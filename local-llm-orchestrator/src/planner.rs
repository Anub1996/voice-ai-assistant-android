//! Hardware-aware model planner.
//!
//! Token generation is memory-bandwidth bound: producing one token streams
//! every *active* weight through the processor once, so
//!
//! ```text
//! tokens/s ≈ efficiency × bandwidth ÷ active-weight bytes
//! ```
//!
//! The planner applies that to each GGUF quantization of a model, checks what
//! fits in RAM/VRAM (weights + quantized KV cache + compute buffers + an OS
//! reserve), splits layers between GPU and CPU when VRAM is short, and picks
//! the highest-quality quantization that fits *and* meets the speed target.
//! When nothing does, it says so plainly and lists models that would.

use serde::{Deserialize, Serialize};

use crate::hardware::Hardware;

const GIB: f64 = 1_073_741_824.0;
/// RAM kept free for the OS, browser, editor... (GiB).
pub const OS_RESERVE_GIB: f64 = 2.5;
/// llama.cpp compute buffers (GiB).
pub const COMPUTE_GIB: f64 = 0.6;
/// VRAM held back for the driver / display (GiB).
pub const VRAM_RESERVE_GIB: f64 = 0.8;
/// Fraction of peak bandwidth llama.cpp achieves during decode.
pub const EFF_CPU: f64 = 0.6;
pub const EFF_GPU: f64 = 0.75;
/// Typical speculative-decoding speed-up range with a same-family draft model.
pub const DRAFT_SPEEDUP: (f64, f64) = (1.4, 1.9);
/// Quantizations below this are offered only if nothing else fits.
const MIN_SANE_BPW: f64 = 2.9;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    /// Total parameters, billions.
    pub total_b: f64,
    /// Parameters used per token, billions (== total for dense models).
    pub active_b: f64,
    pub layers: u32,
    /// KV-cache bytes per token of context at f16.
    pub kv_bytes_per_token: f64,
    /// Hugging Face GGUF repo usable with `llama-server -hf repo:QUANT`.
    #[serde(default)]
    pub hf_repo: Option<String>,
    #[serde(default)]
    pub draft: Option<Draft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Draft {
    pub name: String,
    pub hf_repo: String,
    pub size_gib: f64,
}

impl ModelSpec {
    pub fn is_moe(&self) -> bool {
        self.active_b < self.total_b * 0.9
    }
}

fn kv(layers: u32, kv_heads: u32, head_dim: u32) -> f64 {
    // K and V, f16 (2 bytes).
    2.0 * layers as f64 * kv_heads as f64 * head_dim as f64 * 2.0
}

pub fn presets() -> Vec<ModelSpec> {
    let m = |id: &str,
             name: &str,
             total: f64,
             active: f64,
             layers,
             kvb,
             repo: &str,
             draft: Option<Draft>| {
        ModelSpec {
            id: id.into(),
            name: name.into(),
            total_b: total,
            active_b: active,
            layers,
            kv_bytes_per_token: kvb,
            hf_repo: Some(repo.into()),
            draft,
        }
    };
    let draft = |name: &str, repo: &str, size| {
        Some(Draft {
            name: name.into(),
            hf_repo: repo.into(),
            size_gib: size,
        })
    };
    vec![
        m(
            "gemma-3-27b",
            "Gemma 3 27B (dense)",
            27.0,
            27.0,
            62,
            kv(62, 16, 128),
            "unsloth/gemma-3-27b-it-GGUF",
            draft("Gemma 3 1B", "unsloth/gemma-3-1b-it-GGUF", 0.8),
        ),
        m(
            "qwen3-32b",
            "Qwen3 32B (dense)",
            32.8,
            32.8,
            64,
            kv(64, 8, 128),
            "unsloth/Qwen3-32B-GGUF",
            draft("Qwen3 0.6B", "unsloth/Qwen3-0.6B-GGUF", 0.5),
        ),
        m(
            "qwen3-30b-a3b",
            "Qwen3 30B-A3B (MoE, 3B active)",
            30.5,
            3.3,
            48,
            kv(48, 4, 128),
            "unsloth/Qwen3-30B-A3B-GGUF",
            None,
        ),
        m(
            "mistral-small-24b",
            "Mistral Small 3.2 24B (dense)",
            23.6,
            23.6,
            40,
            kv(40, 8, 128),
            "unsloth/Mistral-Small-3.2-24B-Instruct-2506-GGUF",
            None,
        ),
        m(
            "gemma-3-12b",
            "Gemma 3 12B (dense)",
            12.2,
            12.2,
            48,
            kv(48, 8, 256),
            "unsloth/gemma-3-12b-it-GGUF",
            draft("Gemma 3 1B", "unsloth/gemma-3-1b-it-GGUF", 0.8),
        ),
        m(
            "qwen3-8b",
            "Qwen3 8B (dense)",
            8.2,
            8.2,
            36,
            kv(36, 8, 128),
            "unsloth/Qwen3-8B-GGUF",
            draft("Qwen3 0.6B", "unsloth/Qwen3-0.6B-GGUF", 0.5),
        ),
    ]
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Quant {
    pub name: &'static str,
    /// Average bits per weight.
    pub bpw: f64,
    pub quality: &'static str,
}

pub const QUANTS: &[Quant] = &[
    Quant {
        name: "Q8_0",
        bpw: 8.5,
        quality: "near-lossless",
    },
    Quant {
        name: "Q6_K",
        bpw: 6.56,
        quality: "excellent",
    },
    Quant {
        name: "Q5_K_M",
        bpw: 5.69,
        quality: "very good",
    },
    Quant {
        name: "Q4_K_M",
        bpw: 4.85,
        quality: "good (common default)",
    },
    Quant {
        name: "IQ4_XS",
        bpw: 4.25,
        quality: "good",
    },
    Quant {
        name: "Q3_K_M",
        bpw: 3.91,
        quality: "fair, some loss",
    },
    Quant {
        name: "IQ3_M",
        bpw: 3.66,
        quality: "fair, noticeable loss",
    },
    Quant {
        name: "IQ3_XXS",
        bpw: 3.06,
        quality: "usable, clear loss",
    },
    Quant {
        name: "IQ2_M",
        bpw: 2.7,
        quality: "poor",
    },
    Quant {
        name: "IQ2_XXS",
        bpw: 2.06,
        quality: "very poor",
    },
];

#[derive(Debug, Clone, Deserialize)]
pub struct PlanRequest {
    /// Preset id; ignored when `custom` is given.
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub custom: Option<ModelSpec>,
    /// Defaults to auto-detected hardware.
    #[serde(default)]
    pub hardware: Option<Hardware>,
    #[serde(default = "default_ctx")]
    pub ctx: u32,
    #[serde(default = "default_target")]
    pub target_tps: f64,
}

fn default_ctx() -> u32 {
    4096
}
fn default_target() -> f64 {
    20.0
}

#[derive(Debug, Clone, Serialize)]
pub struct QuantOption {
    pub quant: &'static str,
    pub bpw: f64,
    pub quality: &'static str,
    pub weights_gib: f64,
    pub kv_gib: f64,
    /// RAM this needs (GiB), excluding the OS reserve.
    pub ram_gib: f64,
    /// VRAM this needs (GiB).
    pub vram_gib: f64,
    pub fits: bool,
    pub gpu_layers: u32,
    pub est_tps: f64,
    /// Estimated range with speculative decoding, if a draft model exists.
    pub est_tps_draft: Option<(f64, f64)>,
    pub meets_target: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alternative {
    pub model: String,
    pub name: String,
    pub quant: &'static str,
    pub est_tps: f64,
    pub ram_gib: f64,
    pub meets_target: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub model: ModelSpec,
    pub hardware: Hardware,
    pub ctx: u32,
    pub target_tps: f64,
    pub options: Vec<QuantOption>,
    pub recommended: Option<QuantOption>,
    pub meets_target: bool,
    pub verdict: String,
    /// Bandwidth (GB/s) needed for the target at the recommended quant.
    pub required_bandwidth_gbs: Option<f64>,
    pub command: Option<String>,
    pub command_speculative: Option<String>,
    pub llmo_config: Option<String>,
    pub alternatives: Vec<Alternative>,
}

pub fn find_preset(id: &str) -> Option<ModelSpec> {
    presets().into_iter().find(|m| m.id == id)
}

pub fn plan(req: &PlanRequest, detected: impl FnOnce() -> Hardware) -> Result<Plan, String> {
    let model = match (&req.custom, &req.model) {
        (Some(c), _) => c.clone(),
        (None, Some(id)) => {
            find_preset(id).ok_or_else(|| format!("unknown model preset `{id}`"))?
        }
        (None, None) => return Err("pass `model` (preset id) or `custom`".into()),
    };
    if model.total_b <= 0.0 || model.active_b <= 0.0 || model.layers == 0 {
        return Err("model parameters must be positive".into());
    }
    let hw = req.hardware.clone().unwrap_or_else(detected);
    let ctx = req.ctx.clamp(512, 262_144);
    let target = req.target_tps.max(0.1);

    let options: Vec<QuantOption> = QUANTS
        .iter()
        .map(|q| evaluate(&model, &hw, ctx, target, q))
        .collect();
    let recommended = recommend(&options);
    let meets_target = recommended.as_ref().is_some_and(|r| r.meets_target);

    let alternatives = if meets_target {
        Vec::new()
    } else {
        alternatives(&model, &hw, ctx, target)
    };

    let required_bandwidth_gbs = recommended.as_ref().map(|r| {
        let active_gib = r.weights_gib * model.active_b / model.total_b;
        let eff = if hw.gpu.is_some() { EFF_GPU } else { EFF_CPU };
        (target * active_gib * GIB / 1e9 / eff).round()
    });

    let verdict = verdict(
        &model,
        &hw,
        target,
        recommended.as_ref(),
        required_bandwidth_gbs,
        &alternatives,
    );
    let (command, command_speculative, llmo_config) = match &recommended {
        Some(r) => commands(&model, &hw, ctx, r),
        None => (None, None, None),
    };

    Ok(Plan {
        model,
        hardware: hw,
        ctx,
        target_tps: target,
        options,
        recommended,
        meets_target,
        verdict,
        required_bandwidth_gbs,
        command,
        command_speculative,
        llmo_config,
        alternatives,
    })
}

pub fn evaluate(model: &ModelSpec, hw: &Hardware, ctx: u32, target: f64, q: &Quant) -> QuantOption {
    let weights = model.total_b * 1e9 * q.bpw / 8.0 / GIB;
    let active = model.active_b * 1e9 * q.bpw / 8.0 / GIB;
    // q8_0 KV cache: half of f16.
    let kv_gib = model.kv_bytes_per_token / 2.0 * ctx as f64 / GIB;
    let ram_usable = hw.ram_gb - OS_RESERVE_GIB;

    let (fits, gpu_layers, ram_gib, vram_gib, tps) = if hw.unified_memory {
        let gpu_cap = hw
            .gpu
            .as_ref()
            .map(|g| g.vram_gb)
            .unwrap_or(hw.ram_gb * 0.75);
        let need = weights + kv_gib + COMPUTE_GIB;
        let fits = need <= gpu_cap.min(ram_usable);
        let bw = hw
            .gpu
            .as_ref()
            .map(|g| g.bandwidth_gbs)
            .unwrap_or(hw.cpu_bandwidth_gbs);
        let tps = EFF_GPU * bw * 1e9 / GIB / active;
        (fits, model.layers, need, need, tps)
    } else {
        let vram_usable = hw
            .gpu
            .as_ref()
            .map(|g| (g.vram_gb - VRAM_RESERVE_GIB).max(0.0))
            .unwrap_or(0.0);
        let gpu_budget = vram_usable - kv_gib - COMPUTE_GIB;
        let gpu_layers = if gpu_budget <= 0.0 {
            0
        } else if gpu_budget >= weights {
            model.layers
        } else {
            ((gpu_budget / weights) * model.layers as f64).floor() as u32
        };
        let gpu_frac = gpu_layers as f64 / model.layers as f64;
        let gpu_w = weights * gpu_frac;
        let cpu_w = weights - gpu_w;
        let (ram_gib, vram_gib) = if gpu_layers == 0 {
            (cpu_w + kv_gib + COMPUTE_GIB, 0.0)
        } else {
            (cpu_w + 0.3, gpu_w + kv_gib + COMPUTE_GIB)
        };
        let fits = ram_gib <= ram_usable;
        let cpu_bw = EFF_CPU * hw.cpu_bandwidth_gbs * 1e9 / GIB;
        let gpu_bw = hw
            .gpu
            .as_ref()
            .map(|g| EFF_GPU * g.bandwidth_gbs * 1e9 / GIB)
            .unwrap_or(1.0);
        let secs = active * gpu_frac / gpu_bw + active * (1.0 - gpu_frac) / cpu_bw;
        (fits, gpu_layers, ram_gib, vram_gib, 1.0 / secs)
    };

    let est_tps = round1(tps);
    let est_tps_draft = model
        .draft
        .as_ref()
        .map(|_| (round1(tps * DRAFT_SPEEDUP.0), round1(tps * DRAFT_SPEEDUP.1)));
    QuantOption {
        quant: q.name,
        bpw: q.bpw,
        quality: q.quality,
        weights_gib: round1(weights),
        kv_gib: round2(kv_gib),
        ram_gib: round1(ram_gib),
        vram_gib: round1(vram_gib),
        fits,
        gpu_layers,
        est_tps,
        est_tps_draft,
        meets_target: fits && est_tps >= target,
    }
}

/// Highest-quality option that fits and meets the target; otherwise the
/// highest-quality sane option that fits; otherwise any that fits.
fn recommend(options: &[QuantOption]) -> Option<QuantOption> {
    let sane = |o: &&QuantOption| o.fits && o.bpw >= MIN_SANE_BPW;
    options
        .iter()
        .filter(sane)
        .find(|o| o.meets_target)
        .or_else(|| options.iter().find(sane))
        .or_else(|| options.iter().find(|o| o.fits))
        .cloned()
}

fn alternatives(current: &ModelSpec, hw: &Hardware, ctx: u32, target: f64) -> Vec<Alternative> {
    let mut alts: Vec<Alternative> = presets()
        .into_iter()
        .filter(|m| m.id != current.id)
        .filter_map(|m| {
            let opts: Vec<QuantOption> = QUANTS
                .iter()
                .map(|q| evaluate(&m, hw, ctx, target, q))
                .collect();
            let r = recommend(&opts)?;
            Some(Alternative {
                model: m.id.clone(),
                name: m.name.clone(),
                quant: r.quant,
                est_tps: r.est_tps,
                ram_gib: r.ram_gib,
                meets_target: r.meets_target,
            })
        })
        .collect();
    // Models that meet the target first, then faster ones.
    alts.sort_by(|a, b| {
        b.meets_target
            .cmp(&a.meets_target)
            .then(b.est_tps.total_cmp(&a.est_tps))
    });
    alts
}

fn verdict(
    m: &ModelSpec,
    hw: &Hardware,
    target: f64,
    rec: Option<&QuantOption>,
    required_bw: Option<f64>,
    alts: &[Alternative],
) -> String {
    let Some(r) = rec else {
        return format!(
            "{} does not fit in {:.0} GiB of RAM{} even at 2-bit. Choose a smaller model.",
            m.name,
            hw.ram_gb,
            hw.gpu
                .as_ref()
                .map(|g| format!(" + {:.0} GiB VRAM", g.vram_gb))
                .unwrap_or_default()
        );
    };
    let place = if r.gpu_layers == m.layers && hw.gpu.is_some() {
        "fully on the GPU".to_string()
    } else if r.gpu_layers > 0 {
        format!("with {}/{} layers on the GPU", r.gpu_layers, m.layers)
    } else {
        "on the CPU".to_string()
    };
    if r.meets_target {
        return format!(
            "{} at {} ({:.1} GiB) fits {place} and should generate about {:.0} tokens/s, meeting the {:.0} tok/s target.",
            m.name, r.quant, r.weights_gib, r.est_tps, target
        );
    }
    let bw_now = if hw.unified_memory || r.gpu_layers == m.layers {
        hw.gpu
            .as_ref()
            .map(|g| g.bandwidth_gbs)
            .unwrap_or(hw.cpu_bandwidth_gbs)
    } else {
        hw.cpu_bandwidth_gbs
    };
    let mut s = format!(
        "{} fits at {} ({:.1} GiB, {place}) but will generate only about {:.1} tokens/s, short of {:.0}. \
         Generation speed is limited by memory bandwidth: each token reads ~{:.1} GiB of weights and this machine \
         has ~{:.0} GB/s ({}). Reaching {:.0} tok/s needs ~{:.0} GB/s, e.g. a GPU with enough VRAM for the whole model or an Apple M-series Pro/Max.",
        m.name,
        r.quant,
        r.weights_gib,
        r.est_tps,
        target,
        r.weights_gib * m.active_b / m.total_b,
        bw_now,
        hw.bandwidth_source,
        target,
        required_bw.unwrap_or(0.0)
    );
    if let Some((lo, hi)) = r.est_tps_draft {
        s.push_str(&format!(
            " Speculative decoding with {} could raise it to roughly {lo:.0}–{hi:.0} tok/s.",
            m.draft
                .as_ref()
                .map(|d| d.name.as_str())
                .unwrap_or("a draft model")
        ));
    }
    if let Some(a) = alts.iter().find(|a| a.meets_target) {
        s.push_str(&format!(
            " On this machine {} at {} would reach ~{:.0} tok/s.",
            a.name, a.quant, a.est_tps
        ));
    }
    s
}

fn commands(
    m: &ModelSpec,
    hw: &Hardware,
    ctx: u32,
    r: &QuantOption,
) -> (Option<String>, Option<String>, Option<String>) {
    let model_arg = match &m.hf_repo {
        Some(repo) => vec!["-hf".to_string(), format!("{repo}:{}", r.quant)],
        None => vec![
            "-m".to_string(),
            format!("/models/{}-{}.gguf", m.id, r.quant),
        ],
    };
    let ngl = if hw.gpu.is_none() && !hw.unified_memory {
        0
    } else if r.gpu_layers >= m.layers {
        999
    } else {
        r.gpu_layers
    };
    let mut args = model_arg;
    args.extend(
        [
            "-c",
            &ctx.to_string(),
            "-t",
            &hw.physical_cores.max(1).to_string(),
            "-ngl",
            &ngl.to_string(),
            "-fa",
            "on",
            "-ctk",
            "q8_0",
            "-ctv",
            "q8_0",
            "--host",
            "127.0.0.1",
            "--port",
            "8081",
        ]
        .map(String::from),
    );
    let cmd = format!("llama-server {}", args.join(" "));
    let spec = m.draft.as_ref().map(|d| {
        format!(
            "{cmd} -hfd {}:Q4_K_M -ngld {} --draft-max 16 --draft-min 4",
            d.hf_repo,
            if ngl > 0 { 999 } else { 0 }
        )
    });
    let memory_mb = ((r.ram_gib.max(r.vram_gib)) * 1024.0).ceil() as u64;
    let quoted: Vec<String> = args.iter().map(|a| format!("{a:?}")).collect();
    let toml = format!(
        "[[runtimes]]\nname = \"{id}\"\nbase_url = \"http://127.0.0.1:8081\"\nmodels = [\"{id}\"]\nstartup_timeout_secs = 900  # first run downloads the model\n\n[runtimes.process]\ncommand = \"llama-server\"\nargs = [{args}]\nmemory_mb = {memory_mb}\nidle_timeout_secs = 900\n",
        id = m.id,
        args = quoted.join(", "),
    );
    (Some(cmd), spec, Some(toml))
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::Gpu;

    fn laptop(ram: f64, bw: f64) -> Hardware {
        Hardware {
            os: "linux".into(),
            cpu_name: "test".into(),
            physical_cores: 8,
            logical_cores: 16,
            ram_gb: ram,
            ram_available_gb: ram - 2.0,
            cpu_bandwidth_gbs: bw,
            gpu: None,
            unified_memory: false,
            bandwidth_source: "assumed".into(),
        }
    }

    fn req(model: &str, hw: Hardware) -> PlanRequest {
        PlanRequest {
            model: Some(model.into()),
            custom: None,
            hardware: Some(hw),
            ctx: 4096,
            target_tps: 20.0,
        }
    }

    fn run(model: &str, hw: Hardware) -> Plan {
        plan(&req(model, hw), || unreachable!()).unwrap()
    }

    #[test]
    fn dense_27b_fits_16gb_cpu_but_is_slow() {
        let p = run("gemma-3-27b", laptop(16.0, 60.0));
        let r = p.recommended.as_ref().unwrap();
        assert!(r.fits && r.bpw < 4.0, "needs a 3-bit quant: {}", r.quant);
        assert!(!p.meets_target);
        assert!(r.est_tps > 2.0 && r.est_tps < 8.0, "{}", r.est_tps);
        assert!(p.verdict.contains("memory bandwidth"));
        // Points at a model that does reach the target.
        assert!(
            p.alternatives
                .iter()
                .any(|a| a.model == "qwen3-30b-a3b" && a.meets_target)
        );
        assert!(p.command.unwrap().contains("-ngl 0"));
    }

    #[test]
    fn moe_reaches_20_tps_on_16gb_cpu() {
        let p = run("qwen3-30b-a3b", laptop(16.0, 60.0));
        assert!(p.meets_target, "{}", p.verdict);
        assert!(p.recommended.unwrap().est_tps >= 20.0);
    }

    #[test]
    fn dense_27b_reaches_20_tps_on_16gb_gpu() {
        let mut hw = laptop(16.0, 60.0);
        hw.gpu = Some(Gpu {
            name: "RTX 4060 Ti 16GB".into(),
            vram_gb: 16.0,
            bandwidth_gbs: 288.0,
        });
        let p = run("gemma-3-27b", hw);
        let r = p.recommended.unwrap();
        assert!(p.meets_target, "{}", p.verdict);
        assert_eq!(r.gpu_layers, 62);
        assert!(p.command.unwrap().contains("-ngl 999"));
    }

    #[test]
    fn partial_offload_splits_layers() {
        let mut hw = laptop(32.0, 60.0);
        hw.gpu = Some(Gpu {
            name: "RTX 3060".into(),
            vram_gb: 8.0,
            bandwidth_gbs: 360.0,
        });
        let o = evaluate(
            &find_preset("gemma-3-27b").unwrap(),
            &hw,
            4096,
            20.0,
            &QUANTS[3],
        );
        assert!(o.gpu_layers > 0 && o.gpu_layers < 62);
        let cpu_only = evaluate(
            &find_preset("gemma-3-27b").unwrap(),
            &laptop(32.0, 60.0),
            4096,
            20.0,
            &QUANTS[3],
        );
        assert!(o.est_tps > cpu_only.est_tps);
    }

    #[test]
    fn apple_unified_memory() {
        let mac = |ram: f64, bw: f64, name: &str| {
            let mut hw = laptop(ram, bw);
            hw.unified_memory = true;
            hw.gpu = Some(Gpu {
                name: name.into(),
                vram_gb: ram * 0.75,
                bandwidth_gbs: bw,
            });
            hw
        };
        // M4 Pro: fits comfortably but lands just under 20 tok/s.
        let p = run("gemma-3-27b", mac(24.0, 273.0, "Apple M4 Pro"));
        assert!(!p.meets_target);
        assert!(p.recommended.unwrap().est_tps > 10.0);
        // M4 Max: meets it at a 4-bit quant.
        let p = run("gemma-3-27b", mac(36.0, 546.0, "Apple M4 Max"));
        assert!(p.meets_target, "{}", p.verdict);
        assert!(p.recommended.unwrap().bpw >= 4.0);
    }

    #[test]
    fn too_small_machine() {
        let p = run("qwen3-32b", laptop(6.0, 40.0));
        assert!(p.recommended.is_none());
        assert!(p.verdict.contains("does not fit"));
    }

    #[test]
    fn unknown_preset_errors() {
        assert!(plan(&req("nope", laptop(16.0, 60.0)), || unreachable!()).is_err());
    }
}
