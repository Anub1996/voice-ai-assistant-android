//! Model loading and generation on the CPU with candle's quantized kernels.

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail, ensure};
use candle_core::quantized::gguf_file;
use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_phi3;

use super::arch::{
    gemma3 as quantized_gemma3, llama as quantized_llama, qwen2 as quantized_qwen2,
    qwen3 as quantized_qwen3,
};
use serde::Serialize;

use super::tokenizer::{StreamDecoder, Tokenizer};

/// Architectures (GGUF `general.architecture`) the built-in engine runs.
/// Others (e.g. mixture-of-experts) fall back to llama.cpp.
pub const SUPPORTED: &[&str] = &["llama", "mistral", "qwen2", "qwen3", "gemma3", "phi3"];

pub fn supported(arch: &str) -> bool {
    SUPPORTED.contains(&arch)
}

enum Weights {
    Llama(quantized_llama::ModelWeights),
    Qwen2(quantized_qwen2::ModelWeights),
    Qwen3(quantized_qwen3::ModelWeights),
    Gemma3(quantized_gemma3::ModelWeights),
    Phi3(quantized_phi3::ModelWeights),
}

pub struct Engine {
    pub arch: String,
    pub name: Option<String>,
    pub tok: Tokenizer,
    pub template: Option<String>,
    /// Max tokens (prompt + reply) for this session.
    pub ctx: usize,
    weights: Weights,
    device: Device,
    /// Tokens whose keys/values are currently in the model's KV cache.
    cache: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct GenParams {
    pub max_tokens: usize,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub seed: u64,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    pub stop: Vec<String>,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.7,
            top_p: Some(0.9),
            top_k: Some(40),
            seed: 42,
            repeat_penalty: 1.05,
            repeat_last_n: 64,
            stop: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GenStats {
    pub prompt_tokens: usize,
    /// Prompt tokens actually computed (the rest came from the cache).
    pub prompt_computed: usize,
    pub completion_tokens: usize,
    pub prompt_ms: f64,
    pub gen_ms: f64,
    pub tokens_per_sec: f64,
    pub finish_reason: &'static str,
}

/// Prompt tokens processed per forward pass (bounds attention memory).
const PREFILL_CHUNK: usize = 256;

fn md_ctx(ct: &gguf_file::Content, arch: &str) -> Option<usize> {
    ct.metadata
        .get(&format!("{arch}.context_length"))
        .and_then(|v| v.to_u32().ok())
        .map(|v| v as usize)
}

impl Engine {
    pub fn load(path: &Path, ctx: usize) -> Result<Self> {
        let mut file =
            std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let ct = gguf_file::Content::read(&mut file)
            .map_err(|e| anyhow::anyhow!("reading GGUF: {e}"))?;
        let arch = match ct.metadata.get("general.architecture") {
            Some(gguf_file::Value::String(s)) => s.clone(),
            _ => bail!("GGUF has no general.architecture"),
        };
        ensure!(
            supported(&arch),
            "architecture `{arch}` is not supported by the built-in engine"
        );
        let name = match ct.metadata.get("general.name") {
            Some(gguf_file::Value::String(s)) => Some(s.clone()),
            _ => None,
        };
        let tok = Tokenizer::from_gguf(&ct.metadata)?;
        let template = match ct.metadata.get("tokenizer.chat_template") {
            Some(gguf_file::Value::String(s)) => Some(s.clone()),
            _ => None,
        };
        // Respect both the requested context and what the model supports.
        let model_max = match arch.as_str() {
            "llama" | "mistral" => Some(quantized_llama::MAX_SEQ_LEN),
            a => md_ctx(&ct, a),
        };
        let ctx = model_max.map_or(ctx, |m| ctx.min(m)).max(64);
        let device = Device::Cpu;
        let err = |e: candle_core::Error| anyhow::anyhow!("loading {arch} weights: {e}");
        let weights = match arch.as_str() {
            "llama" | "mistral" => Weights::Llama(
                quantized_llama::ModelWeights::from_gguf(ct, &mut file, &device).map_err(err)?,
            ),
            "qwen2" => Weights::Qwen2(
                quantized_qwen2::ModelWeights::from_gguf(ct, &mut file, &device).map_err(err)?,
            ),
            "qwen3" => Weights::Qwen3(
                quantized_qwen3::ModelWeights::from_gguf(ct, &mut file, &device).map_err(err)?,
            ),
            "gemma3" => Weights::Gemma3(
                quantized_gemma3::ModelWeights::from_gguf(ct, &mut file, &device).map_err(err)?,
            ),
            "phi3" => Weights::Phi3(
                quantized_phi3::ModelWeights::from_gguf(false, ct, &mut file, &device)
                    .map_err(err)?,
            ),
            _ => unreachable!(),
        };
        Ok(Self {
            arch,
            name,
            tok,
            template,
            ctx,
            weights,
            device,
            cache: Vec::new(),
        })
    }

    fn forward(&mut self, tokens: &[u32], pos: usize) -> Result<Vec<f32>> {
        let x = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;
        let logits = match &mut self.weights {
            Weights::Llama(m) => m.forward(&x, pos)?,
            Weights::Qwen2(m) => m.forward(&x, pos)?,
            Weights::Qwen3(m) => m.forward(&x, pos)?,
            Weights::Gemma3(m) => m.forward(&x, pos)?,
            Weights::Phi3(m) => m.forward(&x, pos)?,
        };
        Ok(logits.squeeze(0)?.to_dtype(DType::F32)?.to_vec1::<f32>()?)
    }

    /// Tokens currently held in the KV cache (prompt + generated so far).
    pub fn cached(&self) -> &[u32] {
        &self.cache
    }

    /// Forget the KV cache. Llama/Qwen2/Qwen3 expose a reset; Gemma 3 and
    /// Phi-3 reset themselves when fed from position 0.
    fn reset(&mut self) {
        match &mut self.weights {
            Weights::Llama(m) => m.clear_kv_cache(),
            Weights::Qwen2(m) => m.clear_kv_cache(),
            Weights::Qwen3(m) => m.clear_kv_cache(),
            Weights::Gemma3(_) | Weights::Phi3(_) => {}
        }
        self.cache.clear();
    }

    /// Feed `prompt`, reusing the KV cache for any prefix shared with the
    /// previous call (fast multi-turn chat), and return the next-token logits
    /// plus how many prompt tokens had to be computed.
    pub fn prefill(&mut self, prompt: &[u32]) -> Result<(Vec<f32>, usize)> {
        ensure!(!prompt.is_empty(), "empty prompt");
        let common = self
            .cache
            .iter()
            .zip(prompt)
            .take_while(|(a, b)| a == b)
            .count();
        // Reusable only if the whole cache is a strict prefix of the prompt
        // (caches can't be truncated).
        let start = if common == self.cache.len() && common < prompt.len() {
            common
        } else {
            self.reset();
            0
        };
        let mut logits = Vec::new();
        let mut pos = start;
        for chunk in prompt[start..].chunks(PREFILL_CHUNK) {
            logits = self.forward(chunk, pos)?;
            pos += chunk.len();
            self.cache.extend_from_slice(chunk);
        }
        Ok((logits, prompt.len() - start))
    }

    /// Generate a reply. `on_text` receives decoded text pieces and returns
    /// `false` to stop early (e.g. the client disconnected).
    pub fn generate(
        &mut self,
        prompt: &[u32],
        p: &GenParams,
        mut on_text: impl FnMut(&str) -> bool,
    ) -> Result<GenStats> {
        ensure!(
            prompt.len() < self.ctx,
            "prompt is {} tokens but the context is {}; shorten the conversation",
            prompt.len(),
            self.ctx
        );
        let max_new = p.max_tokens.min(self.ctx - prompt.len()).max(1);
        let t0 = Instant::now();
        let (mut logits, computed) = self.prefill(prompt)?;
        let prompt_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let sampling = if p.temperature <= 0.0 {
            Sampling::ArgMax
        } else {
            match (p.top_k, p.top_p) {
                (Some(k), Some(tp)) => Sampling::TopKThenTopP {
                    k,
                    p: tp,
                    temperature: p.temperature,
                },
                (Some(k), None) => Sampling::TopK {
                    k,
                    temperature: p.temperature,
                },
                (None, Some(tp)) => Sampling::TopP {
                    p: tp,
                    temperature: p.temperature,
                },
                (None, None) => Sampling::All {
                    temperature: p.temperature,
                },
            }
        };
        let mut lp = LogitsProcessor::from_sampling(p.seed, sampling);
        let mut dec = StreamDecoder::default();
        let mut text = String::new();
        let mut emitted = 0usize;
        let mut generated = 0usize;
        let mut finish = "length";
        let mut hit_stop_string = false;
        let t1 = Instant::now();
        // Hold back enough text that a stop string split across tokens is caught.
        let hold = p.stop.iter().map(|s| s.len()).max().unwrap_or(0);

        loop {
            let mut l = Tensor::new(logits.as_slice(), &self.device)?;
            if p.repeat_penalty != 1.0 {
                let recent: Vec<u32> = self
                    .cache
                    .iter()
                    .rev()
                    .take(p.repeat_last_n)
                    .copied()
                    .collect();
                l = candle_transformers::utils::apply_repeat_penalty(
                    &l,
                    p.repeat_penalty,
                    &recent,
                )?;
            }
            let next = lp.sample(&l)?;
            if self.tok.stop.contains(&next) {
                finish = "stop";
                break;
            }
            generated += 1;
            text.push_str(&dec.push(&self.tok, next));
            if let Some(idx) = p.stop.iter().filter_map(|s| text.find(s.as_str())).min() {
                text.truncate(idx);
                hit_stop_string = true;
                finish = "stop";
                break;
            }
            let mut safe = text.len().saturating_sub(hold);
            while !text.is_char_boundary(safe) {
                safe -= 1;
            }
            if safe > emitted {
                if !on_text(&text[emitted..safe]) {
                    finish = "cancelled";
                    break;
                }
                emitted = safe;
            }
            if generated >= max_new {
                break;
            }
            let pos = self.cache.len();
            logits = self.forward(&[next], pos)?;
            self.cache.push(next);
        }
        if finish != "cancelled" {
            if !hit_stop_string {
                text.push_str(&dec.finish());
            }
            if text.len() > emitted {
                on_text(&text[emitted..]);
            }
        }
        let gen_ms = t1.elapsed().as_secs_f64() * 1000.0;
        Ok(GenStats {
            prompt_tokens: prompt.len(),
            prompt_computed: computed,
            completion_tokens: generated,
            prompt_ms,
            gen_ms,
            tokens_per_sec: if gen_ms > 0.0 {
                generated as f64 * 1000.0 / gen_ms
            } else {
                0.0
            },
            finish_reason: finish,
        })
    }
}
