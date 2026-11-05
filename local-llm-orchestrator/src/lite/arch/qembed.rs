//! Token embeddings that stay quantized in memory.
//!
//! Upstream candle expands the whole embedding table to f32 at load time
//! (e.g. 151,936 × 1,024 × 4 B ≈ 620 MB for Qwen3-0.6B, or ≈ 670 MB for
//! Gemma-3-270M). This keeps the GGUF bytes and dequantizes only the rows a
//! step actually looks up, as llama.cpp does.

use candle_core::quantized::{GgmlDType, QTensor, ggml_file::qtensor_from_ggml};
use candle_core::{DType, Device, Module, Result, Tensor, bail};

#[derive(Clone)]
pub struct QEmbedding {
    data: std::sync::Arc<Vec<u8>>,
    dtype: GgmlDType,
    rows: usize,
    dim: usize,
    row_bytes: usize,
    device: Device,
}

impl QEmbedding {
    pub fn new(q: &QTensor) -> Result<Self> {
        let (rows, dim) = q.shape().dims2()?;
        let dtype = q.dtype();
        let bs = dtype.block_size();
        if dim % bs != 0 {
            bail!("embedding width {dim} is not a multiple of the {dtype:?} block size {bs}");
        }
        let row_bytes = dim / bs * dtype.type_size();
        let data = q.data()?.into_owned();
        if data.len() != rows * row_bytes {
            bail!(
                "embedding data is {} bytes, expected {}",
                data.len(),
                rows * row_bytes
            );
        }
        Ok(Self {
            data: std::sync::Arc::new(data),
            dtype,
            rows,
            dim,
            row_bytes,
            device: q.device(),
        })
    }
}

impl std::fmt::Debug for QEmbedding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "QEmbedding({} x {} {:?})",
            self.rows, self.dim, self.dtype
        )
    }
}

impl Module for QEmbedding {
    fn forward(&self, ids: &Tensor) -> Result<Tensor> {
        let mut shape = ids.dims().to_vec();
        let flat: Vec<u32> = ids.flatten_all()?.to_dtype(DType::U32)?.to_vec1()?;
        let mut buf = Vec::with_capacity(flat.len() * self.row_bytes);
        for &id in &flat {
            let id = id as usize;
            if id >= self.rows {
                bail!("token id {id} is outside the vocabulary ({})", self.rows);
            }
            buf.extend_from_slice(&self.data[id * self.row_bytes..(id + 1) * self.row_bytes]);
        }
        let q = qtensor_from_ggml(self.dtype, &buf, vec![flat.len(), self.dim], &self.device)?;
        shape.push(self.dim);
        q.dequantize(&self.device)?.reshape(shape)
    }
}
