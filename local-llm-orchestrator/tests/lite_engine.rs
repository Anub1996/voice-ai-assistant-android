//! Correctness tests for the built-in Rust engine (llmo-lite).

mod support;

use std::time::Duration;

use candle_core::quantized::GgmlDType;
use llmo::lite::template::Msg;
use llmo::lite::{Engine, GenParams};
use serde_json::{Value, json};
use support::{Spec, python_numpy, reference_logits, write_model};

fn max_rel_diff(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vocab size");
    let scale = b.iter().fold(1f32, |m, x| m.max(x.abs()));
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0f32, f32::max)
        / scale
}

fn argmax(v: &[f32]) -> usize {
    v.iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0
}

const PROMPT: &str = "<|im_start|>user\nhello world, 42!<|im_end|>\n<|im_start|>assistant\n";

fn check_against_numpy(arch: &'static str) {
    let Some(py) = python_numpy() else {
        eprintln!("skipping: python3 with numpy not available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), Spec::small(arch), GgmlDType::F32, 7);
    let mut e = Engine::load(&path, 512).unwrap();
    let tokens = e.tok.encode(PROMPT, true);
    assert!(tokens.len() > 10);
    let (ours, computed) = e.prefill(&tokens).unwrap();
    assert_eq!(computed, tokens.len());
    let theirs = reference_logits(py, dir.path(), &tokens);
    let d = max_rel_diff(&ours, &theirs);
    assert!(
        d < 1e-3,
        "{arch}: logits differ from the numpy reference by {d}"
    );
    assert_eq!(
        argmax(&ours),
        argmax(&theirs),
        "{arch}: different top token"
    );
}

#[test]
fn llama_matches_numpy_reference() {
    check_against_numpy("llama");
}

#[test]
fn qwen2_matches_numpy_reference() {
    check_against_numpy("qwen2");
}

#[test]
fn qwen3_matches_numpy_reference() {
    check_against_numpy("qwen3");
}

/// Reusing the KV cache for a shared prefix must give identical results to
/// recomputing everything (multi-turn chat relies on this).
#[test]
fn prefix_cache_is_exact() {
    for arch in ["llama", "qwen2", "qwen3", "gemma3"] {
        let dir = tempfile::tempdir().unwrap();
        let path = write_model(dir.path(), Spec::small(arch), GgmlDType::F32, 11);
        let mut full = Engine::load(&path, 512).unwrap();
        let mut inc = Engine::load(&path, 512).unwrap();
        let tokens = full.tok.encode(PROMPT, true);
        let (a, _) = full.prefill(&tokens).unwrap();
        let split = tokens.len() - 5;
        inc.prefill(&tokens[..split]).unwrap();
        let (b, computed) = inc.prefill(&tokens).unwrap();
        assert_eq!(computed, 5, "{arch}: only the new suffix is computed");
        let d = max_rel_diff(&b, &a);
        assert!(d < 1e-4, "{arch}: cached vs full differ by {d}");
        // A diverging prompt resets the cache and still matches.
        let mut other = tokens.clone();
        other[3] = (other[3] + 1) % 200;
        let (c, computed) = inc.prefill(&other).unwrap();
        assert_eq!(computed, other.len());
        let (d2, _) = full.prefill(&other).unwrap();
        assert!(max_rel_diff(&c, &d2) < 1e-4, "{arch}: reset path");
    }
}

/// Gemma 3's sliding-window layers must limit attention the same way in
/// one-pass prefill and in token-by-token decoding past the window (64 in
/// the test model). Upstream candle skipped the mask for single tokens.
#[test]
fn gemma3_sliding_window_consistent_past_window() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), Spec::small("gemma3"), GgmlDType::F32, 21);
    let mut full = Engine::load(&path, 512).unwrap();
    let mut step = Engine::load(&path, 512).unwrap();
    let tokens: Vec<u32> = (0..110u32).map(|i| 1 + (i * 37) % 250).collect();
    let (a, _) = full.prefill(&tokens).unwrap();
    step.prefill(&tokens[..40]).unwrap();
    let mut last = Vec::new();
    for n in 41..=tokens.len() {
        let (l, computed) = step.prefill(&tokens[..n]).unwrap();
        assert_eq!(computed, 1);
        last = l;
    }
    let d = max_rel_diff(&last, &a);
    assert!(
        d < 1e-4,
        "token-by-token vs one pass differ by {d} past the sliding window"
    );
}

#[test]
fn quantized_models_track_full_precision() {
    for (dtype, tol) in [(GgmlDType::Q8_0, 0.05), (GgmlDType::Q4K, 0.25)] {
        let dir_f = tempfile::tempdir().unwrap();
        let dir_q = tempfile::tempdir().unwrap();
        let spec = Spec::quantizable("qwen2");
        let pf = write_model(dir_f.path(), spec, GgmlDType::F32, 3);
        let pq = write_model(dir_q.path(), spec, dtype, 3);
        let mut ef = Engine::load(&pf, 512).unwrap();
        let mut eq = Engine::load(&pq, 512).unwrap();
        let tokens = ef.tok.encode(PROMPT, true);
        let (lf, _) = ef.prefill(&tokens).unwrap();
        let (lq, _) = eq.prefill(&tokens).unwrap();
        let d = max_rel_diff(&lq, &lf);
        assert!(d < tol, "{dtype:?}: quantized logits off by {d}");
        let qsize = std::fs::metadata(&pq).unwrap().len();
        let fsize = std::fs::metadata(&pf).unwrap().len();
        assert!(
            qsize * 3 < fsize,
            "{dtype:?} file should be much smaller ({qsize} vs {fsize})"
        );
    }
}

#[test]
fn generation_controls() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), Spec::small("llama"), GgmlDType::F32, 5);
    let mut e = Engine::load(&path, 512).unwrap();
    let prompt = e.tok.encode(PROMPT, true);
    let greedy = GenParams {
        temperature: 0.0,
        max_tokens: 12,
        ..Default::default()
    };
    let run = |e: &mut Engine, p: &GenParams| {
        let mut out = String::new();
        let s = e.generate(&prompt, p, |t| {
            out.push_str(t);
            true
        });
        (out, s.unwrap())
    };
    let (a, sa) = run(&mut e, &greedy);
    let (b, _) = run(&mut e, &greedy);
    assert_eq!(
        a, b,
        "greedy decoding is deterministic (and cache reuse is consistent)"
    );
    assert!(sa.completion_tokens <= 12);
    assert!(sa.finish_reason == "length" || sa.finish_reason == "stop");
    if sa.finish_reason == "length" {
        assert_eq!(sa.completion_tokens, 12);
    }
    // Stop string: cut the reply at the first occurrence of a substring of it.
    if a.chars().count() > 4 {
        let stop: String = a.chars().skip(2).take(2).collect();
        let (c, sc) = run(
            &mut e,
            &GenParams {
                stop: vec![stop.clone()],
                ..greedy.clone()
            },
        );
        assert_eq!(sc.finish_reason, "stop");
        assert!(!c.contains(&stop), "{c:?} contains {stop:?}");
        assert!(a.starts_with(&c));
    }
    // Cancellation from the callback.
    let mut n = 0;
    let s = e.generate(
        &prompt,
        &GenParams {
            max_tokens: 50,
            ..greedy.clone()
        },
        |_| {
            n += 1;
            n < 2
        },
    );
    assert_eq!(s.unwrap().finish_reason, "cancelled");
    // Prompt longer than the context is rejected clearly.
    let mut tiny = Engine::load(&path, 64).unwrap();
    let long = tiny.tok.encode(&"hello ".repeat(100), false);
    let err = tiny
        .generate(&long, &greedy, |_| true)
        .unwrap_err()
        .to_string();
    assert!(err.contains("context"), "{err}");
}

#[test]
fn gemma3_loads_and_generates() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), Spec::small("gemma3"), GgmlDType::F32, 9);
    let mut e = Engine::load(&path, 512).unwrap();
    assert_eq!(e.arch, "gemma3");
    let prompt = llmo::lite::server::build_prompt(
        &e,
        &[Msg {
            role: "user".into(),
            content: "hi".into(),
        }],
    );
    let s = e
        .generate(
            &prompt,
            &GenParams {
                max_tokens: 5,
                temperature: 0.0,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
    assert!(s.completion_tokens >= 1);
}

#[test]
fn unsupported_architecture_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), Spec::small("llama"), GgmlDType::F32, 1);
    // Rewrite the architecture string in place (same length keeps offsets).
    let mut bytes = std::fs::read(&path).unwrap();
    let pos = bytes.windows(5).position(|w| w == b"llama").unwrap();
    bytes[pos..pos + 5].copy_from_slice(b"bloom");
    std::fs::write(&path, &bytes).unwrap();
    let err = Engine::load(&path, 512).err().unwrap().to_string();
    assert!(err.contains("not supported"), "{err}");
}

/// The `llmo run-model` process: OpenAI-compatible, streaming and not.
#[tokio::test]
async fn run_model_server_speaks_openai() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), Spec::small("qwen3"), GgmlDType::Q8_0, 2);
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_llmo"))
        .args([
            "run-model",
            "--model",
            path.to_str().unwrap(),
            "--alias",
            "tiny",
            "--port",
            &port.to_string(),
            "--threads",
            "2",
        ])
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    llmo::net::init_tls();
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let base = format!("http://127.0.0.1:{port}");
    let mut up = false;
    for _ in 0..100 {
        if let Ok(r) = http.get(format!("{base}/v1/models")).send().await {
            let v: Value = r.json().await.unwrap();
            assert_eq!(v["data"][0]["id"], "tiny");
            up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(up, "run-model did not come up");

    let body = json!({ "model": "tiny", "max_tokens": 8, "temperature": 0,
                       "messages": [{ "role": "user", "content": "hello" }] });
    let v: Value = http
        .post(format!("{base}/v1/chat/completions"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["object"], "chat.completion");
    assert!(v["usage"]["completion_tokens"].as_u64().unwrap() >= 1);
    assert!(v["timings"]["predicted_per_second"].as_f64().unwrap() > 0.0);
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .to_string();

    let mut sbody = body.clone();
    sbody["stream"] = json!(true);
    let sse = http
        .post(format!("{base}/v1/chat/completions"))
        .json(&sbody)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(sse.trim_end().ends_with("data: [DONE]"));
    let streamed: String = sse
        .lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .filter(|l| l.starts_with('{'))
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|c| {
            c["choices"][0]["delta"]["content"]
                .as_str()
                .map(String::from)
        })
        .collect();
    assert_eq!(
        streamed, text,
        "streaming and non-streaming agree under greedy decoding"
    );
    assert!(
        sse.contains("\"finish_reason\":\"length\"") || sse.contains("\"finish_reason\":\"stop\"")
    );

    let bad = http
        .post(format!("{base}/v1/chat/completions"))
        .json(&json!({ "messages": "nope" }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 400);
    child.kill().await.unwrap();
}
