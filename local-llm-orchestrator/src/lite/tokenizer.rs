//! Pure-Rust tokenizer built from GGUF metadata (no `tokenizers` crate,
//! no C/C++), covering the two families small chat models use:
//!
//! * **Byte-level BPE** (`tokenizer.ggml.model = "gpt2"`): Qwen 2/2.5/3,
//!   SmolLM2, Llama 3, Mistral Nemo… with the model's pre-tokenizer regex
//!   (`tokenizer.ggml.pre`), mirroring llama.cpp.
//! * **SentencePiece BPE** (`"llama"`): Llama 2, TinyLlama, Gemma, Phi-3…
//!   with byte fallback (`<0xXX>`).
//!
//! Special tokens (control / user-defined, e.g. `<|im_start|>`) are matched
//! literally in prompts so chat templates map to single tokens.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use candle_core::quantized::gguf_file::Value;
use fancy_regex::Regex;

const T_CONTROL: i32 = 3;
const T_USER_DEFINED: i32 = 4;
const T_UNUSED: i32 = 5;
const T_BYTE: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Bpe,
    Spm,
}

pub struct Tokenizer {
    kind: Kind,
    vocab: Vec<String>,
    ids: HashMap<String, u32>,
    merges: HashMap<(String, String), u32>,
    scores: Vec<f32>,
    types: Vec<i32>,
    /// Control/user-defined tokens, longest first, for literal matching.
    specials: Vec<(String, u32)>,
    pre: Vec<Regex>,
    ignore_merges: bool,
    add_space_prefix: bool,
    byte_tokens: Vec<Option<u32>>,
    b2u: [char; 256],
    u2b: HashMap<char, u8>,
    pub bos: Option<u32>,
    pub eos: Option<u32>,
    pub add_bos: bool,
    /// Tokens that end a generation.
    pub stop: Vec<u32>,
    pub unk: Option<u32>,
}

/// GPT-2's reversible byte -> printable-unicode mapping.
fn bytes_to_unicode() -> [char; 256] {
    let mut map = ['\0'; 256];
    let mut n = 0u32;
    for b in 0..=255u32 {
        let printable =
            (33..=126).contains(&b) || (161..=172).contains(&b) || (174..=255).contains(&b);
        map[b as usize] = if printable {
            char::from_u32(b).unwrap()
        } else {
            let c = char::from_u32(256 + n).unwrap();
            n += 1;
            c
        };
    }
    map
}

const GPT2_RE: &str = r"'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+";
const LLAMA3_RE: &str = r"(?:'[sS]|'[tT]|'[rR][eE]|'[vV][eE]|'[mM]|'[lL][lL]|'[dD])|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}{1,3}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+";
const QWEN2_RE: &str = r"(?:'[sS]|'[tT]|'[rR][eE]|'[vV][eE]|'[mM]|'[lL][lL]|'[dD])|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+";

/// Pre-tokenizer regexes per `tokenizer.ggml.pre` (from llama.cpp).
fn pre_regexes(pre: &str) -> (Vec<&'static str>, bool) {
    match pre {
        "llama3" | "llama-bpe" | "llama-v3" | "dbrx" | "smaug-bpe" | "falcon3" | "pixtral"
        | "tekken" => (vec![LLAMA3_RE], true),
        "qwen2" | "deepseek-r1-qwen" | "qwen35" | "hunyuan" => (vec![QWEN2_RE], false),
        "smollm" | "stablelm2" | "refact" | "starcoder" => (vec![r"\p{N}", GPT2_RE], false),
        _ => (vec![GPT2_RE], false),
    }
}

fn md<'a>(m: &'a HashMap<String, Value>, k: &str) -> Option<&'a Value> {
    m.get(k)
}

fn md_u32(m: &HashMap<String, Value>, k: &str) -> Option<u32> {
    match md(m, k)? {
        Value::U32(v) => Some(*v),
        Value::I32(v) => u32::try_from(*v).ok(),
        Value::U64(v) => u32::try_from(*v).ok(),
        Value::I64(v) => u32::try_from(*v).ok(),
        Value::U16(v) => Some(*v as u32),
        _ => None,
    }
}

fn md_bool(m: &HashMap<String, Value>, k: &str) -> Option<bool> {
    match md(m, k)? {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

fn md_strings(m: &HashMap<String, Value>, k: &str) -> Option<Vec<String>> {
    match md(m, k)? {
        Value::Array(a) => a
            .iter()
            .map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// Split `text` by `re`, keeping both matches and the gaps between them.
fn split_keep(re: &Regex, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut last = 0;
    for m in re.find_iter(text).flatten() {
        if m.start() > last {
            out.push(text[last..m.start()].to_string());
        }
        if m.end() > m.start() {
            out.push(m.as_str().to_string());
        }
        last = m.end();
    }
    if last < text.len() {
        out.push(text[last..].to_string());
    }
    out
}

impl Tokenizer {
    pub fn from_gguf(m: &HashMap<String, Value>) -> Result<Self> {
        let model = match md(m, "tokenizer.ggml.model") {
            Some(Value::String(s)) => s.clone(),
            _ => bail!("GGUF has no tokenizer.ggml.model"),
        };
        let kind = match model.as_str() {
            "gpt2" => Kind::Bpe,
            "llama" => Kind::Spm,
            other => bail!("tokenizer type `{other}` is not supported by the built-in engine"),
        };
        let vocab =
            md_strings(m, "tokenizer.ggml.tokens").context("GGUF has no tokenizer.ggml.tokens")?;
        let n = vocab.len();
        let types: Vec<i32> = match md(m, "tokenizer.ggml.token_type") {
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| match v {
                    Value::I32(x) => *x,
                    Value::U32(x) => *x as i32,
                    _ => 1,
                })
                .collect(),
            _ => vec![1; n],
        };
        let scores: Vec<f32> = match md(m, "tokenizer.ggml.scores") {
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| if let Value::F32(x) = v { *x } else { 0.0 })
                .collect(),
            _ => vec![0.0; n],
        };
        let mut merges = HashMap::new();
        if kind == Kind::Bpe {
            for (rank, line) in md_strings(m, "tokenizer.ggml.merges")
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                if let Some((a, b)) = line.split_once(' ') {
                    merges.insert((a.to_string(), b.to_string()), rank as u32);
                }
            }
        }
        let ids: HashMap<String, u32> = vocab
            .iter()
            .enumerate()
            .map(|(i, t)| (t.clone(), i as u32))
            .collect();
        let mut specials: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                let ty = types.get(*i).copied().unwrap_or(1);
                (ty == T_CONTROL || ty == T_USER_DEFINED) && !t.is_empty()
            })
            .map(|(i, t)| (t.clone(), i as u32))
            .collect();
        specials.sort_by_key(|(s, _)| std::cmp::Reverse(s.len()));

        let pre_name = match md(m, "tokenizer.ggml.pre") {
            Some(Value::String(s)) => s.clone(),
            _ => "default".into(),
        };
        let (pats, ignore_merges) = pre_regexes(&pre_name);
        let pre = pats
            .into_iter()
            .map(|p| Regex::new(p).expect("valid pre-tokenizer regex"))
            .collect();

        let mut byte_tokens = vec![None; 256];
        if kind == Kind::Spm {
            for (b, slot) in byte_tokens.iter_mut().enumerate() {
                *slot = ids.get(&format!("<0x{b:02X}>")).copied();
            }
        }
        let b2u = bytes_to_unicode();
        let u2b = b2u.iter().enumerate().map(|(b, c)| (*c, b as u8)).collect();

        let bos = md_u32(m, "tokenizer.ggml.bos_token_id");
        let eos = md_u32(m, "tokenizer.ggml.eos_token_id");
        let mut stop: Vec<u32> = [
            eos,
            md_u32(m, "tokenizer.ggml.eot_token_id"),
            md_u32(m, "tokenizer.ggml.eom_token_id"),
        ]
        .into_iter()
        .flatten()
        .collect();
        for marker in [
            "<|im_end|>",
            "<|eot_id|>",
            "<|end_of_text|>",
            "<end_of_turn>",
            "<|end|>",
            "<|endoftext|>",
            "<|return|>",
            "</s>",
        ] {
            if let Some(&id) = ids.get(marker)
                && !stop.contains(&id)
            {
                stop.push(id);
            }
        }
        Ok(Self {
            kind,
            add_bos: md_bool(m, "tokenizer.ggml.add_bos_token").unwrap_or(kind == Kind::Spm),
            add_space_prefix: md_bool(m, "tokenizer.ggml.add_space_prefix")
                .unwrap_or(kind == Kind::Spm),
            unk: md_u32(m, "tokenizer.ggml.unknown_token_id").or_else(|| ids.get("<unk>").copied()),
            vocab,
            ids,
            merges,
            scores,
            types,
            specials,
            pre,
            ignore_merges,
            byte_tokens,
            b2u,
            u2b,
            bos,
            eos,
            stop,
        })
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    pub fn token_text(&self, id: u32) -> Option<&str> {
        self.vocab.get(id as usize).map(String::as_str)
    }

    pub fn id_of(&self, text: &str) -> Option<u32> {
        self.ids.get(text).copied()
    }

    /// Encode text. With `parse_special`, control tokens written literally
    /// (e.g. `<|im_start|>`) become single tokens. BOS is not added here.
    pub fn encode(&self, text: &str, parse_special: bool) -> Vec<u32> {
        let mut out = Vec::new();
        let mut first = true;
        for (seg, special) in self.split_specials(text, parse_special) {
            match special {
                Some(id) => out.push(id),
                None => match self.kind {
                    Kind::Bpe => self.encode_bpe(&seg, &mut out),
                    Kind::Spm => self.encode_spm(&seg, first, &mut out),
                },
            }
            first = false;
        }
        out
    }

    fn split_specials(&self, text: &str, parse: bool) -> Vec<(String, Option<u32>)> {
        if !parse || self.specials.is_empty() {
            return vec![(text.to_string(), None)];
        }
        let mut first_bytes = [false; 256];
        for (s, _) in &self.specials {
            first_bytes[s.as_bytes()[0] as usize] = true;
        }
        let mut out = Vec::new();
        let mut rest = text;
        let mut buf = String::new();
        'outer: while !rest.is_empty() {
            if first_bytes[rest.as_bytes()[0] as usize] {
                for (s, id) in &self.specials {
                    if rest.starts_with(s.as_str()) {
                        if !buf.is_empty() {
                            out.push((std::mem::take(&mut buf), None));
                        }
                        out.push((String::new(), Some(*id)));
                        rest = &rest[s.len()..];
                        continue 'outer;
                    }
                }
            }
            let c = rest.chars().next().unwrap();
            buf.push(c);
            rest = &rest[c.len_utf8()..];
        }
        if !buf.is_empty() {
            out.push((buf, None));
        }
        out
    }

    fn encode_bpe(&self, text: &str, out: &mut Vec<u32>) {
        let mut pieces = vec![text.to_string()];
        for re in &self.pre {
            pieces = pieces.iter().flat_map(|p| split_keep(re, p)).collect();
        }
        for piece in pieces {
            let mapped: String = piece.bytes().map(|b| self.b2u[b as usize]).collect();
            if self.ignore_merges
                && let Some(&id) = self.ids.get(&mapped)
            {
                out.push(id);
                continue;
            }
            let mut syms: Vec<String> = mapped.chars().map(|c| c.to_string()).collect();
            loop {
                let mut best: Option<(u32, usize)> = None;
                for i in 0..syms.len().saturating_sub(1) {
                    if let Some(&r) = self.merges.get(&(syms[i].clone(), syms[i + 1].clone()))
                        && best.is_none_or(|(br, _)| r < br)
                    {
                        best = Some((r, i));
                    }
                }
                let Some((_, i)) = best else { break };
                let merged = format!("{}{}", syms[i], syms[i + 1]);
                syms[i] = merged;
                syms.remove(i + 1);
            }
            for s in syms {
                match self.ids.get(&s) {
                    Some(&id) => out.push(id),
                    None => {
                        for c in s.chars() {
                            if let Some(&id) = self.ids.get(&c.to_string()) {
                                out.push(id);
                            } else if let Some(u) = self.unk {
                                out.push(u);
                            }
                        }
                    }
                }
            }
        }
    }

    fn encode_spm(&self, text: &str, first: bool, out: &mut Vec<u32>) {
        if text.is_empty() {
            return;
        }
        let mut t = String::with_capacity(text.len() + 3);
        if self.add_space_prefix && first && !text.starts_with(' ') {
            t.push('▁');
        }
        t.push_str(&text.replace(' ', "▁"));
        let mut syms: Vec<String> = t.chars().map(|c| c.to_string()).collect();
        loop {
            let mut best: Option<(f32, usize)> = None;
            for i in 0..syms.len().saturating_sub(1) {
                let cand = format!("{}{}", syms[i], syms[i + 1]);
                if let Some(&id) = self.ids.get(&cand) {
                    let s = self.scores.get(id as usize).copied().unwrap_or(0.0);
                    if best.is_none_or(|(bs, _)| s > bs) {
                        best = Some((s, i));
                    }
                }
            }
            let Some((_, i)) = best else { break };
            let merged = format!("{}{}", syms[i], syms[i + 1]);
            syms[i] = merged;
            syms.remove(i + 1);
        }
        for s in syms {
            match self.ids.get(&s) {
                Some(&id) => out.push(id),
                None => {
                    for b in s.bytes() {
                        if let Some(id) = self.byte_tokens[b as usize].or(self.unk) {
                            out.push(id)
                        }
                    }
                }
            }
        }
    }

    pub fn is_control(&self, id: u32) -> bool {
        self.types.get(id as usize).copied() == Some(T_CONTROL)
    }

    /// Raw bytes of one token (control tokens render as nothing).
    pub fn token_bytes(&self, id: u32) -> Vec<u8> {
        let Some(t) = self.vocab.get(id as usize) else {
            return Vec::new();
        };
        let ty = self.types.get(id as usize).copied().unwrap_or(1);
        if ty == T_CONTROL || ty == T_UNUSED {
            return Vec::new();
        }
        match self.kind {
            Kind::Bpe => {
                if ty == T_USER_DEFINED {
                    return t.as_bytes().to_vec();
                }
                let mut v = Vec::with_capacity(t.len());
                for c in t.chars() {
                    match self.u2b.get(&c) {
                        Some(b) => v.push(*b),
                        None => v.extend_from_slice(c.to_string().as_bytes()),
                    }
                }
                v
            }
            Kind::Spm => {
                if ty == T_BYTE
                    && t.len() == 6
                    && t.starts_with("<0x")
                    && let Ok(b) = u8::from_str_radix(&t[3..5], 16)
                {
                    return vec![b];
                }
                t.replace('▁', " ").into_bytes()
            }
        }
    }

    pub fn decode(&self, ids: &[u32]) -> String {
        let mut bytes = Vec::new();
        for &id in ids {
            bytes.extend(self.token_bytes(id));
        }
        let s = String::from_utf8_lossy(&bytes).into_owned();
        if self.kind == Kind::Spm && self.add_space_prefix {
            s.strip_prefix(' ').map(String::from).unwrap_or(s)
        } else {
            s
        }
    }
}

/// Incremental decoder that only emits complete UTF-8 characters.
#[derive(Default)]
pub struct StreamDecoder {
    pending: Vec<u8>,
    started: bool,
}

impl StreamDecoder {
    pub fn push(&mut self, tok: &Tokenizer, id: u32) -> String {
        self.pending.extend(tok.token_bytes(id));
        let valid = match std::str::from_utf8(&self.pending) {
            Ok(_) => self.pending.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(), // incomplete char at the end
            Err(_) => self.pending.len(),                         // invalid: emit lossily
        };
        let chunk: Vec<u8> = self.pending.drain(..valid).collect();
        let mut s = String::from_utf8_lossy(&chunk).into_owned();
        if !self.started && !s.is_empty() {
            self.started = true;
            if tok.kind == Kind::Spm && tok.add_space_prefix {
                s = s.strip_prefix(' ').map(String::from).unwrap_or(s);
            }
        }
        s
    }

    pub fn finish(&mut self) -> String {
        String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn arr_s(v: &[&str]) -> Value {
        Value::Array(v.iter().map(|s| Value::String(s.to_string())).collect())
    }

    /// A small byte-level BPE vocabulary: all 256 byte symbols, a few merges,
    /// and ChatML specials.
    pub fn bpe_metadata(pre: &str) -> HashMap<String, Value> {
        let b2u = bytes_to_unicode();
        let mut tokens: Vec<String> = b2u.iter().map(|c| c.to_string()).collect();
        let merges = [
            "h e", "l l", "he ll", "hell o", "Ġ w", "Ġw o", "Ġwo r", "Ġwor l", "Ġworl d",
        ];
        for m in merges {
            tokens.push(m.replace(' ', ""));
        }
        let mut types = vec![1i32; tokens.len()];
        for s in ["<|im_start|>", "<|im_end|>", "<|endoftext|>"] {
            tokens.push(s.into());
            types.push(T_CONTROL);
        }
        let mut m = HashMap::new();
        m.insert("tokenizer.ggml.model".into(), Value::String("gpt2".into()));
        m.insert("tokenizer.ggml.pre".into(), Value::String(pre.into()));
        m.insert(
            "tokenizer.ggml.tokens".into(),
            Value::Array(tokens.iter().map(|t| Value::String(t.clone())).collect()),
        );
        m.insert(
            "tokenizer.ggml.token_type".into(),
            Value::Array(types.into_iter().map(Value::I32).collect()),
        );
        m.insert("tokenizer.ggml.merges".into(), arr_s(&merges));
        m.insert(
            "tokenizer.ggml.eos_token_id".into(),
            Value::U32(tokens.len() as u32 - 1),
        );
        m
    }

    #[test]
    fn bpe_roundtrip_merges_and_specials() {
        let t = Tokenizer::from_gguf(&bpe_metadata("qwen2")).unwrap();
        let ids = t.encode("<|im_start|>hello world 123 ünï<|im_end|>", true);
        assert_eq!(ids[0], t.id_of("<|im_start|>").unwrap());
        assert_eq!(*ids.last().unwrap(), t.id_of("<|im_end|>").unwrap());
        assert!(ids.contains(&t.id_of("hello").unwrap()), "merged: {ids:?}");
        assert!(ids.contains(&t.id_of("Ġworld").unwrap()));
        // qwen2 splits digits individually.
        let d = t.encode("123", false);
        assert_eq!(d.len(), 3);
        // Round trip, control tokens skipped in output.
        assert_eq!(t.decode(&ids), "hello world 123 ünï");
        assert!(t.stop.contains(&t.id_of("<|im_end|>").unwrap()));
        assert!(t.stop.contains(&t.id_of("<|endoftext|>").unwrap()));
        // Without parse_special the marker is just text.
        let raw = t.encode("<|im_end|>", false);
        assert!(raw.len() > 1);
        assert_eq!(t.decode(&raw), "<|im_end|>");
    }

    #[test]
    fn llama3_pre_groups_digits_by_three() {
        let t = Tokenizer::from_gguf(&bpe_metadata("llama-bpe")).unwrap();
        // "1234567" -> "123" "456" "7" pieces -> 7 byte tokens either way,
        // but pieces never merge across groups; round trip must hold.
        let ids = t.encode("abc 1234567\n\n  x", false);
        assert_eq!(t.decode(&ids), "abc 1234567\n\n  x");
    }

    #[test]
    fn stream_decoder_handles_split_utf8() {
        let t = Tokenizer::from_gguf(&bpe_metadata("default")).unwrap();
        let ids = t.encode("é✓", false); // multi-byte chars split over byte tokens
        let mut d = StreamDecoder::default();
        let mut s = String::new();
        for id in ids {
            s.push_str(&d.push(&t, id));
        }
        s.push_str(&d.finish());
        assert_eq!(s, "é✓");
    }

    pub fn spm_metadata() -> HashMap<String, Value> {
        let mut tokens: Vec<String> = vec!["<unk>".into(), "<s>".into(), "</s>".into()];
        let mut types = vec![2, T_CONTROL, T_CONTROL];
        let mut scores = vec![0.0f32, 0.0, 0.0];
        for b in 0..=255u8 {
            tokens.push(format!("<0x{b:02X}>"));
            types.push(T_BYTE);
            scores.push(0.0);
        }
        for (i, w) in [
            "▁", "h", "e", "l", "o", "w", "r", "d", "he", "ll", "hell", "hello", "▁hello", "▁w",
            "or", "▁wor", "▁world", "ld",
        ]
        .iter()
        .enumerate()
        {
            tokens.push(w.to_string());
            types.push(1);
            scores.push(i as f32); // longer pieces score higher
        }
        let mut m = HashMap::new();
        m.insert("tokenizer.ggml.model".into(), Value::String("llama".into()));
        m.insert(
            "tokenizer.ggml.tokens".into(),
            Value::Array(tokens.iter().map(|t| Value::String(t.clone())).collect()),
        );
        m.insert(
            "tokenizer.ggml.token_type".into(),
            Value::Array(types.into_iter().map(Value::I32).collect()),
        );
        m.insert(
            "tokenizer.ggml.scores".into(),
            Value::Array(scores.into_iter().map(Value::F32).collect()),
        );
        m.insert("tokenizer.ggml.bos_token_id".into(), Value::U32(1));
        m.insert("tokenizer.ggml.eos_token_id".into(), Value::U32(2));
        m
    }

    #[test]
    fn spm_merges_by_score_with_byte_fallback() {
        let t = Tokenizer::from_gguf(&spm_metadata()).unwrap();
        assert!(t.add_bos);
        let ids = t.encode("hello world!", false);
        assert_eq!(ids[0], t.id_of("▁hello").unwrap());
        assert_eq!(ids[1], t.id_of("▁world").unwrap());
        // "!" is not in the vocab: byte fallback <0x21>.
        assert_eq!(ids[2], t.id_of("<0x21>").unwrap());
        assert_eq!(t.decode(&ids), "hello world!");
        // Non-ASCII through byte fallback round-trips too.
        let ids = t.encode("héllo", false);
        assert_eq!(t.decode(&ids), "héllo");
    }
}
