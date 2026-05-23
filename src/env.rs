//! 路徑、Homebrew、依賴檢查、執行緒數。
use std::path::PathBuf;
use std::process::Command;

pub fn home() -> PathBuf {
    dirs_home().unwrap_or_else(|| PathBuf::from("."))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn data_dir() -> PathBuf {
    if let Some(v) = std::env::var_os("TW_HOME") {
        return PathBuf::from(v);
    }
    home().join(".local/share/t-whisper")
}
pub fn models_dir() -> PathBuf {
    data_dir().join("models")
}
pub fn legacy_models_dir() -> PathBuf {
    home().join("models/whisper")
}
pub fn config_dir() -> PathBuf {
    if let Some(v) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(v).join("t-whisper");
    }
    home().join(".config/t-whisper")
}
pub fn config_path() -> PathBuf {
    config_dir().join("config")
}
pub fn default_outdir() -> PathBuf {
    home().join("whisper-transcripts")
}

/// Homebrew 安裝前綴（Apple Silicon /opt/homebrew、Intel /usr/local）。
pub fn brew_prefix() -> Option<String> {
    if let Ok(out) = Command::new("brew").arg("--prefix").output()
        && out.status.success()
    {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !p.is_empty() {
            return Some(p);
        }
    }
    for c in ["/opt/homebrew", "/usr/local"] {
        if std::path::Path::new(c).join("bin/brew").is_file() {
            return Some(c.to_string());
        }
    }
    None
}

/// 指令是否存在於 PATH。
pub fn has_cmd(cmd: &str) -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let p = dir.join(cmd);
            if p.is_file() {
                return true;
            }
        }
    }
    false
}

/// 效能核心數，上限 8。
pub fn threads() -> i32 {
    let mut n = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(4) as i32;
    if let Ok(out) = Command::new("sysctl")
        .args(["-n", "hw.perflevel0.logicalcpu"])
        .output()
        && let Ok(v) = String::from_utf8_lossy(&out.stdout).trim().parse::<i32>()
        && v > 0
    {
        n = v;
    }
    n.clamp(1, 8)
}
