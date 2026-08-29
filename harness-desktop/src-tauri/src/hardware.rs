use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::{Disks, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub os: String,
    pub architecture: String,
    pub cpu_brand: String,
    pub logical_cpu_count: usize,
    pub physical_cpu_count: Option<usize>,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub available_disk_bytes: u64,
    pub gpu: Vec<GpuInfo>,
    pub local_runtime_candidates: Vec<LocalRuntimeCandidate>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub vendor: String,
    pub model: String,
    pub memory_bytes: Option<u64>,
    pub unified_memory: bool,
    pub backend: Option<String>,
    pub estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRuntimeCandidate {
    pub kind: String,
    pub executable_path: String,
    pub managed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareTier {
    Tier0,
    Tier1,
    Tier2,
    Tier3,
    Tier4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAiRecommendation {
    pub tier: HardwareTier,
    pub recommend_install: bool,
    pub execution_mode: String,
    pub maximum_model_memory_bytes: u64,
    pub purposes: Vec<String>,
    pub reasons: Vec<String>,
    pub catalog_selection_required: bool,
}

pub fn analyze_hardware() -> HardwareProfile {
    let mut system = System::new_all();
    system.refresh_all();
    let disks = Disks::new_with_refreshed_list();
    let available_disk_bytes = disks.iter().map(|disk| disk.available_space()).sum();
    let total_memory_bytes = system.total_memory();
    let mut gpu = Vec::new();
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        gpu.push(GpuInfo {
            vendor: "Apple".into(),
            model: "Apple Silicon integrated GPU".into(),
            memory_bytes: Some(total_memory_bytes),
            unified_memory: true,
            backend: Some("Metal".into()),
            estimated: true,
        });
    }
    HardwareProfile {
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        cpu_brand: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_owned())
            .unwrap_or_else(|| "Unknown CPU".into()),
        logical_cpu_count: system.cpus().len(),
        physical_cpu_count: system.physical_core_count(),
        total_memory_bytes,
        available_memory_bytes: system.available_memory(),
        available_disk_bytes,
        gpu,
        local_runtime_candidates: detect_local_runtimes(),
        detected_at: chrono::Utc::now(),
    }
}

pub fn recommend_local_ai(profile: &HardwareProfile) -> LocalAiRecommendation {
    const GIB: u64 = 1024 * 1024 * 1024;
    let memory = profile.total_memory_bytes;
    let usable = memory.saturating_mul(55) / 100;
    let has_accelerator = !profile.gpu.is_empty();
    let tier = if memory < 6 * GIB || profile.available_disk_bytes < 4 * GIB {
        HardwareTier::Tier0
    } else if memory < 12 * GIB {
        HardwareTier::Tier1
    } else if memory < 24 * GIB {
        HardwareTier::Tier2
    } else if has_accelerator && memory < 48 * GIB {
        HardwareTier::Tier3
    } else if has_accelerator {
        HardwareTier::Tier4
    } else {
        HardwareTier::Tier2
    };
    let recommend_install = tier != HardwareTier::Tier0;
    let execution_mode = if has_accelerator {
        "gpu_or_hybrid"
    } else {
        "cpu"
    }
    .into();
    let mut reasons = vec![format!(
        "Recommendation is constrained to approximately 55% of {} bytes of physical memory",
        memory
    )];
    if !has_accelerator {
        reasons.push(
            "No supported GPU was confirmed; CPU inference remains optional and valid".into(),
        );
    }
    if !recommend_install {
        reasons.push(
            "Available memory or disk is below the conservative local runtime threshold".into(),
        );
    }
    LocalAiRecommendation {
        tier,
        recommend_install,
        execution_mode,
        maximum_model_memory_bytes: usable,
        purposes: vec![
            "routing".into(),
            "summarization".into(),
            "classification".into(),
        ],
        reasons,
        // A signed, current catalog chooses the model; model names are deliberately not hardcoded.
        catalog_selection_required: true,
    }
}

fn detect_local_runtimes() -> Vec<LocalRuntimeCandidate> {
    [
        ("ollama", "ollama"),
        ("llama.cpp-cli", "llama-cli"),
        ("llama.cpp-server", "llama-server"),
    ]
    .into_iter()
    .filter_map(|(kind, binary)| {
        find_on_path(binary).map(|path| LocalRuntimeCandidate {
            kind: kind.into(),
            executable_path: path.to_string_lossy().into_owned(),
            managed: false,
        })
    })
    .collect()
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into())
            .split(';')
            .map(str::to_owned)
            .collect()
    } else {
        vec![String::new()]
    };
    for directory in std::env::split_paths(&path) {
        for extension in &extensions {
            let candidate = directory.join(format!("{binary}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(memory_gib: u64, disk_gib: u64, accelerated: bool) -> HardwareProfile {
        HardwareProfile {
            os: "test".into(),
            architecture: "test".into(),
            cpu_brand: "test".into(),
            logical_cpu_count: 4,
            physical_cpu_count: Some(2),
            total_memory_bytes: memory_gib * 1024 * 1024 * 1024,
            available_memory_bytes: memory_gib * 512 * 1024 * 1024,
            available_disk_bytes: disk_gib * 1024 * 1024 * 1024,
            gpu: accelerated
                .then(|| GpuInfo {
                    vendor: "test".into(),
                    model: "test".into(),
                    memory_bytes: None,
                    unified_memory: false,
                    backend: Some("test".into()),
                    estimated: false,
                })
                .into_iter()
                .collect(),
            local_runtime_candidates: vec![],
            detected_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn gpu_absence_still_recommends_cpu_when_memory_allows() {
        let recommendation = recommend_local_ai(&profile(16, 100, false));
        assert!(recommendation.recommend_install);
        assert_eq!(recommendation.execution_mode, "cpu");
        assert_eq!(recommendation.tier, HardwareTier::Tier2);
    }

    #[test]
    fn constrained_machine_declines_install_without_error() {
        let recommendation = recommend_local_ai(&profile(4, 100, false));
        assert!(!recommendation.recommend_install);
        assert_eq!(recommendation.tier, HardwareTier::Tier0);
    }
}
