//! Test helpers: write small random-weight GGUF models (any of the built-in
//! engine's architectures) with a working byte-level BPE tokenizer and a
//! ChatML chat template, and dump raw f32 weights for the numpy reference.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use candle_core::quantized::gguf_file::{self, Value};
use candle_core::quantized::{GgmlDType, QTensor};
use candle_core::{Device, Tensor};

#[derive(Clone, Copy)]
pub struct Spec {
    pub arch: &'static str,
    pub layers: usize,
    pub embd: usize,
    pub heads: usize,
    pub kv_heads: usize,
    pub head_dim: usize,
    pub ffn: usize,
    pub tied: bool,
    /// Vocabulary size (0 = just the test tokenizer's ~270 tokens).
    pub vocab: usize,
}

impl Spec {
    pub fn small(arch: &'static str) -> Self {
        Spec {
            arch,
            layers: 2,
            embd: 64,
            heads: 4,
            kv_heads: 2,
            head_dim: 16,
            ffn: 128,
            tied: arch == "qwen2",
            vocab: 0,
        }
    }
    /// Dimensions that are multiples of 256 so Q4_K/Q8_0 can quantize them.
    pub fn quantizable(arch: &'static str) -> Self {
        Spec {
            arch,
            layers: 2,
            embd: 256,
            heads: 4,
            kv_heads: 2,
            head_dim: 64,
            ffn: 512,
            tied: false,
            vocab: 0,
        }
    }
}

pub const EPS: f32 = 1e-6;
pub const THETA: f32 = 10_000.0;
pub const CHATML: &str = "{%- for message in messages %}{{- '<|im_start|>' + message['role'] + '\\n' + message['content'] + '<|im_end|>\\n' }}{%- endfor %}{%- if add_generation_prompt %}{{- '<|im_start|>assistant\\n' }}{%- endif %}";

fn bytes_to_unicode() -> Vec<char> {
    let mut n = 0u32;
    (0..=255u32)
        .map(|b| {
            if (33..=126).contains(&b) || (161..=172).contains(&b) || (174..=255).contains(&b) {
                char::from_u32(b).unwrap()
            } else {
                n += 1;
                char::from_u32(255 + n).unwrap()
            }
        })
        .collect()
}

/// Tokenizer metadata; returns (metadata, vocab size). `pad_to` adds
/// placeholder tokens to reach a real model's vocabulary size.
pub fn tokenizer_md_padded(pad_to: usize) -> (Vec<(String, Value)>, usize) {
    let mut tokens: Vec<String> = bytes_to_unicode().iter().map(|c| c.to_string()).collect();
    let merges = [
        "h e", "l l", "he ll", "hell o", "Ġ w", "Ġw o", "Ġwo r", "Ġwor l", "Ġworl d",
    ];
    for m in merges {
        tokens.push(m.replace(' ', ""));
    }
    let mut types = vec![1i32; tokens.len()];
    while tokens.len() + 3 < pad_to {
        tokens.push(format!("<pad{}>", tokens.len()));
        types.push(5);
    }
    for s in ["<|im_start|>", "<|im_end|>", "<|endoftext|>"] {
        tokens.push(s.into());
        types.push(3);
    }
    let n = tokens.len();
    let md = vec![
        ("tokenizer.ggml.model".into(), Value::String("gpt2".into())),
        ("tokenizer.ggml.pre".into(), Value::String("qwen2".into())),
        (
            "tokenizer.ggml.tokens".into(),
            Value::Array(tokens.into_iter().map(Value::String).collect()),
        ),
        (
            "tokenizer.ggml.token_type".into(),
            Value::Array(types.into_iter().map(Value::I32).collect()),
        ),
        (
            "tokenizer.ggml.merges".into(),
            Value::Array(
                merges
                    .iter()
                    .map(|m| Value::String(m.to_string()))
                    .collect(),
            ),
        ),
        (
            "tokenizer.ggml.eos_token_id".into(),
            Value::U32(n as u32 - 1),
        ),
        (
            "tokenizer.chat_template".into(),
            Value::String(CHATML.into()),
        ),
    ];
    (md, n)
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) as f32 * 2.0 - 1.0
    }
    fn vec(&mut self, n: usize, scale: f32, offset: f32) -> Vec<f32> {
        (0..n).map(|_| offset + self.next() * scale).collect()
    }
}

/// Write `model.gguf` into `dir` (plus `*.bin` f32 dumps and `spec.json`).
/// 2-D weights are stored as `dtype`; norms and biases stay F32.
pub fn tokenizer_md() -> (Vec<(String, Value)>, usize) {
    tokenizer_md_padded(0)
}

pub fn write_model(dir: &Path, spec: Spec, dtype: GgmlDType, seed: u64) -> PathBuf {
    write_model_opts(dir, spec, dtype, seed, true)
}

/// `dump`: also write raw f32 weights for the numpy reference.
pub fn write_model_opts(
    dir: &Path,
    spec: Spec,
    dtype: GgmlDType,
    seed: u64,
    dump_f32: bool,
) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let (tok_md, vocab) = tokenizer_md_padded(spec.vocab);
    let a = spec.arch;
    let mut md: Vec<(String, Value)> = vec![
        ("general.architecture".into(), Value::String(a.into())),
        ("general.name".into(), Value::String(format!("test-{a}"))),
        (format!("{a}.block_count"), Value::U32(spec.layers as u32)),
        (
            format!("{a}.embedding_length"),
            Value::U32(spec.embd as u32),
        ),
        (
            format!("{a}.feed_forward_length"),
            Value::U32(spec.ffn as u32),
        ),
        (
            format!("{a}.attention.head_count"),
            Value::U32(spec.heads as u32),
        ),
        (
            format!("{a}.attention.head_count_kv"),
            Value::U32(spec.kv_heads as u32),
        ),
        (
            format!("{a}.attention.layer_norm_rms_epsilon"),
            Value::F32(EPS),
        ),
        (format!("{a}.rope.freq_base"), Value::F32(THETA)),
        (
            format!("{a}.rope.dimension_count"),
            Value::U32(spec.head_dim as u32),
        ),
        (format!("{a}.context_length"), Value::U32(512)),
        (
            format!("{a}.attention.key_length"),
            Value::U32(spec.head_dim as u32),
        ),
        (
            format!("{a}.attention.value_length"),
            Value::U32(spec.head_dim as u32),
        ),
    ];
    if a == "gemma3" {
        md.push(("gemma3.attention.sliding_window".into(), Value::U32(64)));
    }
    md.extend(tok_md);

    let mut rng = Rng(seed | 1);
    let mut f32s: BTreeMap<String, (Vec<usize>, Vec<f32>)> = BTreeMap::new();
    let mut mat = |name: String, rows: usize, cols: usize, rng: &mut Rng| {
        let scale = 1.0 / (cols as f32).sqrt();
        f32s.insert(
            name,
            (vec![rows, cols], rng.vec(rows * cols, scale * 1.7, 0.0)),
        );
    };
    let (e, h, kv, hd, f) = (
        spec.embd,
        spec.heads,
        spec.kv_heads,
        spec.head_dim,
        spec.ffn,
    );
    mat("token_embd.weight".into(), vocab, e, &mut rng);
    if !spec.tied {
        mat("output.weight".into(), vocab, e, &mut rng);
    }
    for l in 0..spec.layers {
        let p = format!("blk.{l}.");
        mat(format!("{p}attn_q.weight"), h * hd, e, &mut rng);
        mat(format!("{p}attn_k.weight"), kv * hd, e, &mut rng);
        mat(format!("{p}attn_v.weight"), kv * hd, e, &mut rng);
        mat(format!("{p}attn_output.weight"), e, h * hd, &mut rng);
        mat(format!("{p}ffn_gate.weight"), f, e, &mut rng);
        mat(format!("{p}ffn_up.weight"), f, e, &mut rng);
        mat(format!("{p}ffn_down.weight"), e, f, &mut rng);
    }
    let mut vecs: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    vecs.insert("output_norm.weight".into(), rng.vec(e, 0.1, 1.0));
    for l in 0..spec.layers {
        let p = format!("blk.{l}.");
        vecs.insert(format!("{p}attn_norm.weight"), rng.vec(e, 0.1, 1.0));
        vecs.insert(format!("{p}ffn_norm.weight"), rng.vec(e, 0.1, 1.0));
        if a == "qwen2" {
            vecs.insert(format!("{p}attn_q.bias"), rng.vec(h * hd, 0.05, 0.0));
            vecs.insert(format!("{p}attn_k.bias"), rng.vec(kv * hd, 0.05, 0.0));
            vecs.insert(format!("{p}attn_v.bias"), rng.vec(kv * hd, 0.05, 0.0));
        }
        if a == "qwen3" || a == "gemma3" {
            vecs.insert(format!("{p}attn_q_norm.weight"), rng.vec(hd, 0.1, 1.0));
            vecs.insert(format!("{p}attn_k_norm.weight"), rng.vec(hd, 0.1, 1.0));
        }
        if a == "gemma3" {
            vecs.insert(
                format!("{p}post_attention_norm.weight"),
                rng.vec(e, 0.1, 1.0),
            );
            vecs.insert(format!("{p}post_ffw_norm.weight"), rng.vec(e, 0.1, 1.0));
        }
    }

    let dev = Device::Cpu;
    let mut tensors: Vec<(String, QTensor)> = Vec::new();
    let mut shapes = serde_json::Map::new();
    for (name, (shape, data)) in &f32s {
        let t = Tensor::from_vec(data.clone(), shape.as_slice(), &dev).unwrap();
        tensors.push((name.clone(), QTensor::quantize(&t, dtype).unwrap()));
        if dump_f32 {
            dump(dir, name, data);
        }
        shapes.insert(name.clone(), serde_json::json!(shape));
    }
    for (name, data) in &vecs {
        let t = Tensor::from_vec(data.clone(), data.len(), &dev).unwrap();
        tensors.push((name.clone(), QTensor::quantize(&t, GgmlDType::F32).unwrap()));
        if dump_f32 {
            dump(dir, name, data);
        }
        shapes.insert(name.clone(), serde_json::json!([data.len()]));
    }
    let path = dir.join("model.gguf");
    let mut file = std::fs::File::create(&path).unwrap();
    let md_refs: Vec<(&str, &Value)> = md.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let t_refs: Vec<(&str, &QTensor)> = tensors.iter().map(|(k, v)| (k.as_str(), v)).collect();
    gguf_file::write(&mut file, &md_refs, &t_refs).unwrap();
    let spec_json = serde_json::json!({
        "arch": a, "layers": spec.layers, "heads": h, "kv_heads": kv, "head_dim": hd,
        "eps": EPS, "theta": THETA, "tensors": shapes,
    });
    std::fs::write(dir.join("spec.json"), spec_json.to_string()).unwrap();
    path
}

fn dump(dir: &Path, name: &str, data: &[f32]) {
    let bytes: Vec<u8> = data.iter().flat_map(|x| x.to_le_bytes()).collect();
    std::fs::write(dir.join(format!("{name}.bin")), bytes).unwrap();
}

pub fn python_numpy() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|p| {
        std::process::Command::new(p)
            .args(["-c", "import numpy"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Next-token logits from the numpy reference.
pub fn reference_logits(py: &str, dir: &Path, tokens: &[u32]) -> Vec<f32> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/lite_ref.py");
    let ids: Vec<String> = tokens.iter().map(|t| t.to_string()).collect();
    let out = std::process::Command::new(py)
        .arg(script)
        .arg(dir)
        .arg(ids.join(","))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reference failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
