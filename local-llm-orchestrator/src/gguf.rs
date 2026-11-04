//! Minimal, defensive GGUF header reader.
//!
//! Reads only the metadata key/values and tensor descriptors (not weights)
//! to learn what the planner needs about a downloaded model: layer count,
//! KV-cache geometry and, for mixture-of-experts models, which share of the
//! weights is active per token. Inputs are untrusted downloads, so every
//! length is bounded and malformed files produce errors, never panics or
//! huge allocations.

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 4] = b"GGUF";
const MAX_STRING: u64 = 1 << 24; // 16 MiB
const MAX_KV: u64 = 1 << 20;
const MAX_TENSORS: u64 = 1 << 20;
const MAX_DIMS: u32 = 8;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GgufInfo {
    pub architecture: Option<String>,
    pub name: Option<String>,
    pub block_count: Option<u64>,
    pub context_length: Option<u64>,
    pub embedding_length: Option<u64>,
    pub head_count: Option<u64>,
    pub head_count_kv: Option<u64>,
    pub key_length: Option<u64>,
    pub value_length: Option<u64>,
    pub expert_count: Option<u64>,
    pub expert_used_count: Option<u64>,
    pub file_type: Option<u64>,
    /// Sum of all tensor element counts.
    pub total_params: u64,
    /// Elements in routed-expert tensors (`*_exps*`).
    pub expert_params: u64,
}

impl GgufInfo {
    pub fn is_moe(&self) -> bool {
        self.expert_count.unwrap_or(0) > 1 && self.expert_params > 0
    }

    /// Share of weights read per token (1.0 for dense models).
    pub fn active_fraction(&self) -> f64 {
        if !self.is_moe() || self.total_params == 0 {
            return 1.0;
        }
        let n = self.expert_count.unwrap_or(1) as f64;
        let used = self.expert_used_count.unwrap_or(1).min(n as u64) as f64;
        let inactive = self.expert_params as f64 * (1.0 - used / n);
        ((self.total_params as f64 - inactive) / self.total_params as f64).clamp(0.01, 1.0)
    }

    /// f16 KV-cache bytes per token of context, if the geometry is known.
    pub fn kv_bytes_per_token(&self) -> Option<f64> {
        let layers = self.block_count? as f64;
        let heads = self.head_count.unwrap_or(0);
        let kv_heads = self.head_count_kv.or(self.head_count)? as f64;
        let default_dim = self
            .embedding_length
            .zip((heads > 0).then_some(heads))
            .map(|(e, h)| e / h);
        let k = self.key_length.or(default_dim)? as f64;
        let v = self.value_length.or(self.key_length).or(default_dim)? as f64;
        Some(layers * kv_heads * (k + v) * 2.0)
    }
}

enum Val {
    U(u64),
    I(i64),
    S(String),
    /// First element of a small integer array (e.g. per-layer head counts).
    ArrFirstInt(u64),
    Other,
}

struct R<T: Read + Seek> {
    r: T,
}

impl<T: Read + Seek> R<T> {
    fn u8(&mut self) -> Result<u8> {
        let mut b = [0; 1];
        self.r.read_exact(&mut b)?;
        Ok(b[0])
    }
    fn u32(&mut self) -> Result<u32> {
        let mut b = [0; 4];
        self.r.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }
    fn u64(&mut self) -> Result<u64> {
        let mut b = [0; 8];
        self.r.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
    fn skip(&mut self, n: u64) -> Result<()> {
        let n = i64::try_from(n).context("skip too large")?;
        self.r.seek_relative(n)?;
        Ok(())
    }
    fn string(&mut self) -> Result<String> {
        let len = self.u64()?;
        ensure!(len <= MAX_STRING, "string too long ({len} bytes)");
        let mut buf = vec![0; len as usize];
        self.r.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
    fn skip_string(&mut self) -> Result<()> {
        let len = self.u64()?;
        ensure!(len <= MAX_STRING, "string too long ({len} bytes)");
        self.skip(len)
    }

    fn scalar_size(ty: u32) -> Option<u64> {
        Some(match ty {
            0 | 1 | 7 => 1,
            2 | 3 => 2,
            4..=6 => 4,
            10..=12 => 8,
            _ => return None,
        })
    }

    fn int(&mut self, ty: u32) -> Result<Val> {
        Ok(match ty {
            0 => Val::U(self.u8()? as u64),
            1 => Val::I(self.u8()? as i8 as i64),
            2 => {
                let mut b = [0; 2];
                self.r.read_exact(&mut b)?;
                Val::U(u16::from_le_bytes(b) as u64)
            }
            3 => {
                let mut b = [0; 2];
                self.r.read_exact(&mut b)?;
                Val::I(i16::from_le_bytes(b) as i64)
            }
            4 => Val::U(self.u32()? as u64),
            5 => Val::I(self.u32()? as i32 as i64),
            10 => Val::U(self.u64()?),
            11 => Val::I(self.u64()? as i64),
            _ => bail!("not an integer type {ty}"),
        })
    }

    fn value(&mut self, ty: u32) -> Result<Val> {
        match ty {
            0..=5 | 10 | 11 => self.int(ty),
            6 | 7 | 12 => {
                self.skip(Self::scalar_size(ty).unwrap())?;
                Ok(Val::Other)
            }
            8 => Ok(Val::S(self.string()?)),
            9 => {
                let et = self.u32()?;
                let n = self.u64()?;
                if et == 8 {
                    ensure!(n <= 1 << 24, "array too long");
                    for _ in 0..n {
                        self.skip_string()?;
                    }
                    return Ok(Val::Other);
                }
                let size = Self::scalar_size(et).with_context(|| format!("bad array type {et}"))?;
                ensure!(n <= 1 << 28, "array too long");
                if n > 0 && matches!(et, 0..=5 | 10 | 11) {
                    let first = match self.int(et)? {
                        Val::U(u) => u,
                        Val::I(i) => i.max(0) as u64,
                        _ => 0,
                    };
                    self.skip((n - 1) * size)?;
                    return Ok(Val::ArrFirstInt(first));
                }
                self.skip(n * size)?;
                Ok(Val::Other)
            }
            _ => bail!("unknown GGUF value type {ty}"),
        }
    }
}

/// Read a GGUF file's header. For split models pass every part to
/// [`read_split`] so tensor totals cover the whole model.
pub fn read(path: &Path) -> Result<GgufInfo> {
    let f = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut info = GgufInfo::default();
    parse(BufReader::with_capacity(1 << 16, f), &mut info, true)
        .with_context(|| format!("reading GGUF header of {}", path.display()))?;
    Ok(info)
}

pub fn read_split(paths: &[std::path::PathBuf]) -> Result<GgufInfo> {
    let first = paths.first().context("no files")?;
    let mut info = read(first)?;
    for p in &paths[1..] {
        let f = File::open(p).with_context(|| format!("opening {}", p.display()))?;
        parse(BufReader::new(f), &mut info, false)?;
    }
    Ok(info)
}

fn parse<T: Read + Seek>(r: T, info: &mut GgufInfo, keep_meta: bool) -> Result<()> {
    let mut r = R { r };
    let mut magic = [0; 4];
    r.r.read_exact(&mut magic)?;
    ensure!(&magic == MAGIC, "not a GGUF file");
    let version = r.u32()?;
    ensure!(
        (2..=3).contains(&version),
        "unsupported GGUF version {version}"
    );
    let n_tensors = r.u64()?;
    let n_kv = r.u64()?;
    ensure!(
        n_tensors <= MAX_TENSORS && n_kv <= MAX_KV,
        "implausible header counts"
    );

    let mut kvs: Vec<(String, Val)> = Vec::new();
    for _ in 0..n_kv {
        let key = r.string()?;
        let ty = r.u32()?;
        let v = r.value(ty)?;
        if keep_meta && !matches!(v, Val::Other) {
            kvs.push((key, v));
        }
    }
    if keep_meta {
        let get_s = |k: &str| {
            kvs.iter()
                .find(|(kk, _)| kk == k)
                .and_then(|(_, v)| match v {
                    Val::S(s) => Some(s.clone()),
                    _ => None,
                })
        };
        let arch = get_s("general.architecture");
        let get_u = |suffix: &str| {
            let key = format!("{}.{suffix}", arch.as_deref().unwrap_or(""));
            kvs.iter()
                .find(|(kk, _)| *kk == key)
                .and_then(|(_, v)| match v {
                    Val::U(u) | Val::ArrFirstInt(u) => Some(*u),
                    Val::I(i) => u64::try_from(*i).ok(),
                    _ => None,
                })
        };
        info.block_count = get_u("block_count");
        info.context_length = get_u("context_length");
        info.embedding_length = get_u("embedding_length");
        info.head_count = get_u("attention.head_count");
        info.head_count_kv = get_u("attention.head_count_kv");
        info.key_length = get_u("attention.key_length");
        info.value_length = get_u("attention.value_length");
        info.expert_count = get_u("expert_count");
        info.expert_used_count = get_u("expert_used_count");
        info.file_type = kvs
            .iter()
            .find(|(k, _)| k == "general.file_type")
            .and_then(|(_, v)| match v {
                Val::U(u) => Some(*u),
                _ => None,
            });
        info.name = get_s("general.name");
        info.architecture = arch;
    }

    for _ in 0..n_tensors {
        let name = r.string()?;
        let n_dims = r.u32()?;
        ensure!(n_dims <= MAX_DIMS, "tensor `{name}` has {n_dims} dims");
        let mut elems: u64 = 1;
        for _ in 0..n_dims {
            elems = elems.saturating_mul(r.u64()?);
        }
        let _ty = r.u32()?;
        let _offset = r.u64()?;
        info.total_params = info.total_params.saturating_add(elems);
        if name.contains("_exps") {
            info.expert_params = info.expert_params.saturating_add(elems);
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Tiny GGUF writer for tests.
    pub struct W(pub Vec<u8>);
    impl W {
        pub fn new(n_tensors: u64, n_kv: u64) -> Self {
            let mut w = W(Vec::new());
            w.0.extend_from_slice(MAGIC);
            w.0.extend_from_slice(&3u32.to_le_bytes());
            w.0.extend_from_slice(&n_tensors.to_le_bytes());
            w.0.extend_from_slice(&n_kv.to_le_bytes());
            w
        }
        pub fn s(&mut self, s: &str) -> &mut Self {
            self.0.extend_from_slice(&(s.len() as u64).to_le_bytes());
            self.0.extend_from_slice(s.as_bytes());
            self
        }
        pub fn kv_str(&mut self, k: &str, v: &str) -> &mut Self {
            self.s(k);
            self.0.extend_from_slice(&8u32.to_le_bytes());
            self.s(v)
        }
        pub fn kv_u32(&mut self, k: &str, v: u32) -> &mut Self {
            self.s(k);
            self.0.extend_from_slice(&4u32.to_le_bytes());
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        pub fn kv_str_array(&mut self, k: &str, items: &[&str]) -> &mut Self {
            self.s(k);
            self.0.extend_from_slice(&9u32.to_le_bytes());
            self.0.extend_from_slice(&8u32.to_le_bytes());
            self.0
                .extend_from_slice(&(items.len() as u64).to_le_bytes());
            for i in items {
                self.s(i);
            }
            self
        }
        pub fn kv_f32_array(&mut self, k: &str, n: u64) -> &mut Self {
            self.s(k);
            self.0.extend_from_slice(&9u32.to_le_bytes());
            self.0.extend_from_slice(&6u32.to_le_bytes());
            self.0.extend_from_slice(&n.to_le_bytes());
            self.0.extend(std::iter::repeat_n(0u8, (n * 4) as usize));
            self
        }
        pub fn tensor(&mut self, name: &str, dims: &[u64]) -> &mut Self {
            self.s(name);
            self.0.extend_from_slice(&(dims.len() as u32).to_le_bytes());
            for d in dims {
                self.0.extend_from_slice(&d.to_le_bytes());
            }
            self.0.extend_from_slice(&12u32.to_le_bytes());
            self.0.extend_from_slice(&0u64.to_le_bytes());
            self
        }
    }

    /// A small fake MoE model: 2 layers, 8 experts, 2 active.
    pub fn fake_moe_gguf() -> Vec<u8> {
        let mut w = W::new(4, 11);
        w.kv_str("general.architecture", "qwen3moe")
            .kv_str("general.name", "Fake MoE")
            .kv_u32("qwen3moe.block_count", 2)
            .kv_u32("qwen3moe.context_length", 32768)
            .kv_u32("qwen3moe.embedding_length", 256)
            .kv_u32("qwen3moe.attention.head_count", 8)
            .kv_u32("qwen3moe.attention.head_count_kv", 2)
            .kv_u32("qwen3moe.expert_count", 8)
            .kv_u32("qwen3moe.expert_used_count", 2)
            .kv_str_array("tokenizer.ggml.tokens", &["a", "b", "c"])
            .kv_f32_array("tokenizer.ggml.scores", 3);
        w.tensor("token_embd.weight", &[256, 1000])
            .tensor("blk.0.attn_q.weight", &[256, 256])
            .tensor("blk.0.ffn_up_exps.weight", &[256, 512, 8])
            .tensor("blk.1.ffn_up_exps.weight", &[256, 512, 8]);
        w.0
    }

    #[test]
    fn parses_moe_metadata_and_tensors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m.gguf");
        std::fs::write(&p, fake_moe_gguf()).unwrap();
        let i = read(&p).unwrap();
        assert_eq!(i.architecture.as_deref(), Some("qwen3moe"));
        assert_eq!(i.block_count, Some(2));
        assert_eq!(i.head_count_kv, Some(2));
        assert_eq!(i.total_params, 256 * 1000 + 256 * 256 + 2 * 256 * 512 * 8);
        assert_eq!(i.expert_params, 2 * 256 * 512 * 8);
        assert!(i.is_moe());
        // 6/8 of expert weights are idle per token.
        let expected = 1.0 - (2.0 * 256.0 * 512.0 * 8.0 * 0.75) / i.total_params as f64;
        assert!((i.active_fraction() - expected).abs() < 1e-9);
        // 2 layers * 2 kv heads * (32 + 32) dims * 2 bytes
        assert_eq!(i.kv_bytes_per_token(), Some(2.0 * 2.0 * 64.0 * 2.0));
    }

    #[test]
    fn rejects_garbage_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.gguf");
        std::fs::write(&p, b"not a model").unwrap();
        assert!(read(&p).is_err());
        // Truncated file.
        let mut data = fake_moe_gguf();
        data.truncate(60);
        std::fs::write(&p, &data).unwrap();
        assert!(read(&p).is_err());
        // Absurd string length.
        let mut w = W::new(0, 1);
        w.0.extend_from_slice(&u64::MAX.to_le_bytes());
        std::fs::write(&p, &w.0).unwrap();
        assert!(read(&p).unwrap_err().to_string().contains("GGUF"));
    }

    #[test]
    fn dense_model_is_fully_active() {
        let mut w = W::new(1, 1);
        w.kv_str("general.architecture", "llama")
            .tensor("t", &[10, 10]);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("d.gguf");
        std::fs::write(&p, &w.0).unwrap();
        let i = read(&p).unwrap();
        assert_eq!(i.active_fraction(), 1.0);
        assert_eq!(i.total_params, 100);
    }
}
