//! Best-effort detection of the machine's memory, CPU and GPU.
//!
//! Memory bandwidth cannot be measured portably without a benchmark, so it
//! is looked up from the CPU/GPU name where possible and otherwise assumed;
//! `bandwidth_source` says which. Every value can be overridden in the
//! planner.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hardware {
    pub os: String,
    pub cpu_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub ram_gb: f64,
    pub ram_available_gb: f64,
    /// Effective CPU memory bandwidth, GB/s.
    pub cpu_bandwidth_gbs: f64,
    pub gpu: Option<Gpu>,
    /// Apple silicon style: CPU and GPU share one memory pool.
    pub unified_memory: bool,
    /// `detected` (from a known CPU/GPU name) or `assumed`.
    pub bandwidth_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gpu {
    pub name: String,
    pub vram_gb: f64,
    pub bandwidth_gbs: f64,
}

/// Typical dual-channel desktop/laptop memory (between DDR4-3200 and DDR5-5600).
pub const ASSUMED_CPU_BW: f64 = 60.0;

pub fn detect() -> Hardware {
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let mut hw = Hardware {
        os: std::env::consts::OS.to_string(),
        cpu_name: "unknown CPU".into(),
        physical_cores: logical,
        logical_cores: logical,
        ram_gb: 16.0,
        ram_available_gb: 12.0,
        cpu_bandwidth_gbs: ASSUMED_CPU_BW,
        gpu: None,
        unified_memory: false,
        bandwidth_source: "assumed".into(),
    };
    #[cfg(target_os = "linux")]
    linux(&mut hw);
    #[cfg(target_os = "macos")]
    macos(&mut hw);
    #[cfg(windows)]
    windows(&mut hw);
    if !hw.unified_memory {
        hw.gpu = nvidia();
    }
    hw
}

#[cfg(target_os = "linux")]
fn linux(hw: &mut Hardware) {
    if let Ok(mi) = std::fs::read_to_string("/proc/meminfo") {
        let kb = |key: &str| {
            mi.lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
        };
        if let Some(t) = kb("MemTotal:") {
            hw.ram_gb = round1(t / 1_048_576.0);
        }
        if let Some(a) = kb("MemAvailable:") {
            hw.ram_available_gb = round1(a / 1_048_576.0);
        }
    }
    if let Ok(ci) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(name) = ci
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
        {
            hw.cpu_name = name.trim().to_string();
        }
        let mut cores = std::collections::HashSet::new();
        let mut phys = String::new();
        for l in ci.lines() {
            let val = || l.split(':').nth(1).unwrap_or("").trim().to_string();
            if l.starts_with("physical id") {
                phys = val();
            } else if l.starts_with("core id") {
                cores.insert((phys.clone(), val()));
            }
        }
        if !cores.is_empty() {
            hw.physical_cores = cores.len();
        }
    }
}

#[cfg(target_os = "macos")]
fn macos(hw: &mut Hardware) {
    let sysctl = |k: &str| {
        std::process::Command::new("sysctl")
            .args(["-n", k])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if let Some(b) = sysctl("hw.memsize").and_then(|s| s.parse::<f64>().ok()) {
        hw.ram_gb = round1(b / 1_073_741_824.0);
        hw.ram_available_gb = round1(hw.ram_gb * 0.75);
    }
    if let Some(n) = sysctl("hw.physicalcpu").and_then(|s| s.parse().ok()) {
        hw.physical_cores = n;
    }
    if let Some(name) = sysctl("machdep.cpu.brand_string") {
        hw.cpu_name = name.clone();
        if let Some(bw) = apple_bandwidth(&name) {
            hw.unified_memory = true;
            hw.cpu_bandwidth_gbs = bw;
            hw.bandwidth_source = "detected".into();
            // Metal may use ~75% of unified memory by default.
            hw.gpu = Some(Gpu {
                name: name.clone(),
                vram_gb: round1(hw.ram_gb * 0.75),
                bandwidth_gbs: bw,
            });
        }
    }
}

#[cfg(windows)]
fn windows(hw: &mut Hardware) {
    // One PowerShell call: total bytes, free KiB, CPU name, physical cores.
    let script = "$c=Get-CimInstance Win32_ComputerSystem; $o=Get-CimInstance Win32_OperatingSystem; \
                  $p=@(Get-CimInstance Win32_Processor); $c.TotalPhysicalMemory; $o.FreePhysicalMemory; \
                  $p[0].Name; ($p | Measure-Object NumberOfCores -Sum).Sum";
    let Ok(out) = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
    else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if let Some(b) = lines.first().and_then(|v| v.parse::<f64>().ok()) {
        hw.ram_gb = round1(b / 1_073_741_824.0);
    }
    if let Some(kb) = lines.get(1).and_then(|v| v.parse::<f64>().ok()) {
        hw.ram_available_gb = round1(kb / 1_048_576.0);
    }
    if let Some(name) = lines.get(2) {
        hw.cpu_name = name.to_string();
    }
    if let Some(n) = lines.get(3).and_then(|v| v.parse::<usize>().ok()) {
        hw.physical_cores = n.max(1);
    }
}

/// Unified memory bandwidth of Apple silicon chips, GB/s.
pub fn apple_bandwidth(name: &str) -> Option<f64> {
    let n = name.to_ascii_lowercase();
    let generation = ["m1", "m2", "m3", "m4", "m5"]
        .into_iter()
        .find(|g| n.contains(g))?;
    let tier = if n.contains("ultra") {
        "ultra"
    } else if n.contains("max") {
        "max"
    } else if n.contains("pro") {
        "pro"
    } else {
        "base"
    };
    Some(match (generation, tier) {
        (_, "ultra") => 800.0,
        ("m4" | "m5", "max") => 546.0,
        (_, "max") => 400.0,
        ("m3", "pro") => 150.0,
        ("m4" | "m5", "pro") => 273.0,
        (_, "pro") => 200.0,
        ("m1", _) => 68.0,
        ("m4" | "m5", _) => 120.0,
        _ => 100.0,
    })
}

fn nvidia() -> Option<Gpu> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?;
    let (name, mem) = line.rsplit_once(',')?;
    let vram_gb = round1(mem.trim().parse::<f64>().ok()? / 1024.0);
    let name = name.trim().to_string();
    Some(Gpu {
        bandwidth_gbs: gpu_bandwidth(&name),
        name,
        vram_gb,
    })
}

/// Memory bandwidth of common NVIDIA cards, GB/s (300 if unknown).
pub fn gpu_bandwidth(name: &str) -> f64 {
    let n = name.to_ascii_lowercase();
    const TABLE: &[(&str, f64)] = &[
        ("5090", 1792.0),
        ("5080", 960.0),
        ("5070 ti", 896.0),
        ("5070", 672.0),
        ("5060 ti", 448.0),
        ("4090", 1008.0),
        ("4080", 717.0),
        ("4070 ti super", 672.0),
        ("4070 ti", 504.0),
        ("4070", 504.0),
        ("4060 ti", 288.0),
        ("4060", 272.0),
        ("3090", 936.0),
        ("3080", 760.0),
        ("3070", 448.0),
        ("3060 ti", 448.0),
        ("3060", 360.0),
        ("a100", 1555.0),
        ("h100", 3350.0),
        ("l4", 300.0),
        ("t4", 320.0),
    ];
    TABLE
        .iter()
        .find(|(k, _)| n.contains(k))
        .map(|(_, bw)| *bw)
        .unwrap_or(300.0)
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_tiers() {
        assert_eq!(apple_bandwidth("Apple M1"), Some(68.0));
        assert_eq!(apple_bandwidth("Apple M3 Pro"), Some(150.0));
        assert_eq!(apple_bandwidth("Apple M4 Max"), Some(546.0));
        assert_eq!(apple_bandwidth("Apple M2 Ultra"), Some(800.0));
        assert_eq!(apple_bandwidth("Intel(R) Core(TM) i7"), None);
    }

    #[test]
    fn gpu_table_prefers_specific_names() {
        assert_eq!(gpu_bandwidth("NVIDIA GeForce RTX 4060 Ti"), 288.0);
        assert_eq!(gpu_bandwidth("NVIDIA GeForce RTX 4070 Ti SUPER"), 672.0);
        assert_eq!(gpu_bandwidth("Mystery GPU"), 300.0);
    }

    #[test]
    fn detect_is_sane() {
        let hw = detect();
        assert!(hw.ram_gb > 0.0);
        assert!(hw.physical_cores >= 1);
    }
}
