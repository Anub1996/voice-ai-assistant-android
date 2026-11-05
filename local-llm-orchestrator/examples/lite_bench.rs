//! Benchmark the built-in engine on models with real shapes (random
//! weights; speed depends only on shapes and quantization).
//!   cargo run --release --example lite_bench -- gen DIR
//!   cargo run --release --example lite_bench -- run MODEL.gguf [threads]
//!   cargo run --release --example lite_bench -- bw

#[path = "../tests/support/mod.rs"]
mod support;

use std::time::Instant;

use candle_core::quantized::GgmlDType;
use llmo::lite::{Engine, GenParams};
use rayon::prelude::*;
use support::{Spec, write_model_opts};

fn bandwidth_gbs() -> f64 {
    let n = 256 * 1024 * 1024 / 8;
    let v: Vec<u64> = (0..n as u64).collect();
    let mut best = 0f64;
    for _ in 0..5 {
        let t = Instant::now();
        let s: u64 = v
            .par_chunks(1 << 16)
            .map(|c| c.iter().fold(0u64, |a, x| a.wrapping_add(*x)))
            .sum();
        std::hint::black_box(s);
        best = best.max((n * 8) as f64 / t.elapsed().as_secs_f64() / 1e9);
    }
    best
}

fn rss_mb(key: &str) -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|st| {
            st.lines()
                .find(|l| l.starts_with(key))
                .map(|l| l.split_whitespace().nth(1).unwrap().parse::<f64>().unwrap() / 1024.0)
        })
        .unwrap_or(0.0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("gen") => {
            let dir = std::path::PathBuf::from(&args[2]);
            let cases: [(&str, Spec, GgmlDType); 5] = [
                (
                    "smollm2-135m-q8_0",
                    Spec {
                        arch: "llama",
                        layers: 30,
                        embd: 576,
                        heads: 9,
                        kv_heads: 3,
                        head_dim: 64,
                        ffn: 1536,
                        tied: true,
                        vocab: 49152,
                    },
                    GgmlDType::Q8_0,
                ),
                (
                    "qwen2.5-0.5b-q4_0",
                    Spec {
                        arch: "qwen2",
                        layers: 24,
                        embd: 896,
                        heads: 14,
                        kv_heads: 2,
                        head_dim: 64,
                        ffn: 4864,
                        tied: true,
                        vocab: 151936,
                    },
                    GgmlDType::Q4_0,
                ),
                (
                    "qwen3-0.6b-q4_k",
                    Spec {
                        arch: "qwen3",
                        layers: 28,
                        embd: 1024,
                        heads: 16,
                        kv_heads: 8,
                        head_dim: 128,
                        ffn: 3072,
                        tied: true,
                        vocab: 151936,
                    },
                    GgmlDType::Q4K,
                ),
                (
                    "llama-3.2-1b-q4_k",
                    Spec {
                        arch: "llama",
                        layers: 16,
                        embd: 2048,
                        heads: 32,
                        kv_heads: 8,
                        head_dim: 64,
                        ffn: 8192,
                        tied: true,
                        vocab: 128256,
                    },
                    GgmlDType::Q4K,
                ),
                (
                    "qwen2.5-1.5b-q4_k",
                    Spec {
                        arch: "qwen2",
                        layers: 28,
                        embd: 1536,
                        heads: 12,
                        kv_heads: 2,
                        head_dim: 128,
                        ffn: 8960,
                        tied: true,
                        vocab: 151936,
                    },
                    GgmlDType::Q4K,
                ),
            ];
            for (name, spec, dtype) in cases {
                let d = dir.join(name);
                let p = write_model_opts(&d, spec, dtype, 1, false);
                println!(
                    "{} {:.0} MB",
                    p.display(),
                    std::fs::metadata(&p).unwrap().len() as f64 / 1e6
                );
            }
        }
        Some("bw") => {
            llmo::lite::set_threads(0);
            println!("{:.1}", bandwidth_gbs());
        }
        Some("run") => {
            let path = std::path::PathBuf::from(&args[2]);
            let threads: usize = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(0);
            llmo::lite::set_threads(threads);
            let size = std::fs::metadata(&path).unwrap().len();
            let t = Instant::now();
            let mut e = Engine::load(&path, 2048).unwrap();
            let load = t.elapsed().as_secs_f64();
            let prompt: Vec<u32> = (0..200u32).map(|x| 300 + x * 7 % 200).collect();
            let p = GenParams {
                max_tokens: 64,
                temperature: 0.0,
                repeat_penalty: 1.0,
                ..Default::default()
            };
            e.generate(
                &prompt[..16],
                &GenParams {
                    max_tokens: 4,
                    ..p.clone()
                },
                |_| true,
            )
            .unwrap();
            let s = e.generate(&prompt, &p, |_| true).unwrap();
            // Follow-up turn: 30 new tokens on top of the cached conversation.
            let mut turn2 = e.cached().to_vec();
            turn2.extend((0..30u32).map(|x| 400 + x));
            let s2 = e
                .generate(
                    &turn2,
                    &GenParams {
                        max_tokens: 8,
                        ..p.clone()
                    },
                    |_| true,
                )
                .unwrap();
            println!(
                "{:<20} {:>5.0} MB  load {:.1}s  prompt {:>5.0} tok/s  gen {:>5.1} tok/s  follow-up turn computed {} of {} prompt tokens  peak RAM {:.0} MB",
                path.parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy(),
                size as f64 / 1e6,
                load,
                s.prompt_tokens as f64 / (s.prompt_ms / 1000.0),
                s.tokens_per_sec,
                s2.prompt_computed,
                s2.prompt_tokens,
                rss_mb("VmHWM")
            );
        }
        _ => eprintln!("usage: lite_bench gen DIR | run MODEL [threads] | bw"),
    }
}
