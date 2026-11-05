# Third-party notices

`src/lite/arch/{llama,qwen2,qwen3,gemma3}.rs` are adapted from
[candle-transformers](https://github.com/huggingface/candle) 0.11.1
(`src/models/quantized_*.rs`), © the candle authors, licensed under
MIT OR Apache-2.0. Changes are marked `llmo` in the files:
- Token embeddings stay quantized.
- Gemma 3 attention-mask fixes.

Other dependencies are used as libraries under their own licenses; run
`cargo about` or see `Cargo.lock` for the full list.
