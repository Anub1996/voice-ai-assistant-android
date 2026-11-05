//! **llmo-lite**: the built-in, pure-Rust inference engine.
//!
//! An alternative to llama.cpp for low-spec machines: it ships inside the
//! llmo binary (nothing to download), runs quantized GGUF models on the CPU
//! with candle's SIMD kernels, and uses a tokenizer and chat-template
//! renderer written in Rust that read everything from the GGUF file.
//!
//! Supported model families: Llama (incl. SmolLM2, TinyLlama, Llama 3.x),
//! Mistral, Qwen 2/2.5, Qwen 3, Gemma 3, Phi-3. Everything else falls back
//! to llama.cpp.

pub mod arch;
pub mod model;
pub mod server;
pub mod template;
pub mod tokenizer;

pub use model::{Engine, GenParams, GenStats, supported};

/// Use `threads` CPU threads for matrix maths (0 = physical cores).
pub fn set_threads(threads: usize) {
    let n = if threads == 0 {
        crate::models::hardware().physical_cores.max(1)
    } else {
        threads
    };
    // candle sizes some work by RAYON_NUM_THREADS; set it before any use.
    // SAFETY: called once at process start, before other threads exist.
    unsafe { std::env::set_var("RAYON_NUM_THREADS", n.to_string()) };
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(n)
        .build_global();
}
