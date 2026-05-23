//! 模型 registry、路徑解析、下載（curl）、列出、picker。
use crate::{config::Config, env, ui};
use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

/// 已知模型：(名稱, 檔名, 約位元組數)
const MODELS: &[(&str, &str, u64)] = &[
    ("turbo", "ggml-large-v3-turbo.bin", 1_624_555_275),
    ("large-v3", "ggml-large-v3.bin", 3_095_033_483),
];

fn entry(name: &str) -> Option<&'static (&'static str, &'static str, u64)> {
    MODELS.iter().find(|m| m.0 == name)
}

fn url(filename: &str) -> String {
    format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}")
}

/// 找模型本機路徑：TW_MODELS → legacy ~/models/whisper。找不到回 None。
pub fn path(name: &str) -> Option<PathBuf> {
    let (_, file, _) = entry(name)?;
    let a = env::models_dir().join(file);
    if a.is_file() {
        return Some(a);
    }
    let b = env::legacy_models_dir().join(file);
    if b.is_file() {
        return Some(b);
    }
    None
}

/// 解析模型參數（名稱 turbo/large-v3，或直接給 .bin 路徑）→ 本機路徑，必要時下載。
pub fn resolve(model: &str, assume_yes: bool) -> Result<PathBuf> {
    let p = PathBuf::from(model);
    if p.is_file() {
        return Ok(p);
    }
    if let Some(found) = path(model) {
        return Ok(found);
    }
    ensure(model, assume_yes)
}

/// 確保模型存在，必要時（互動或 --yes）下載。
pub fn ensure(name: &str, assume_yes: bool) -> Result<PathBuf> {
    if let Some(found) = path(name) {
        return Ok(found);
    }
    let (_, file, size) =
        entry(name).with_context(|| format!("未知模型：{name}（可用 turbo / large-v3）"))?;
    if !assume_yes {
        ui::warn(&format!(
            "尚未下載模型 {name}（約 {} MB）",
            size / 1024 / 1024
        ));
        eprint!("要現在下載嗎？[Y/n] ");
        io::stderr().flush().ok();
        let mut ans = String::new();
        io::stdin().read_line(&mut ans).ok();
        if ans.trim().eq_ignore_ascii_case("n") {
            bail!("需要模型才能轉錄（t-whisper models pull {name}）");
        }
    }
    std::fs::create_dir_all(env::models_dir())?;
    let dest = env::models_dir().join(file);
    ui::info(&format!("下載 {name} → {}", dest.display()));
    let status = Command::new("curl")
        .args(["-L", "--fail", "-o"])
        .arg(&dest)
        .arg(url(file))
        .status()
        .context("執行 curl 失敗")?;
    if !status.success() {
        let _ = std::fs::remove_file(&dest);
        bail!("下載失敗，請檢查網路後重試 t-whisper models pull {name}");
    }
    // 校驗大小（容許 1% 誤差）
    let got = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    if *size > 0 && got < size * 99 / 100 {
        let _ = std::fs::remove_file(&dest);
        bail!("模型下載不完整（{got} / {size} bytes），重試 t-whisper models pull {name}");
    }
    ui::ok(&format!("模型就緒：{name}"));
    Ok(dest)
}

pub fn list() {
    ui::info(&format!(
        "模型狀態（存放：{}）：",
        env::models_dir().display()
    ));
    for (name, _, size) in MODELS {
        match path(name) {
            Some(p) => ui::ok(&format!("{name:<9} {}", p.display())),
            None => ui::dim(&format!(
                "⬜ {name:<9} 未下載（約 {} MB）",
                size / 1024 / 1024
            )),
        }
    }
}

pub fn remove(name: &str) -> Result<()> {
    let (_, file, _) = entry(name).with_context(|| format!("未知模型：{name}"))?;
    let managed = env::models_dir().join(file);
    if managed.is_file() {
        std::fs::remove_file(&managed)?;
        ui::ok(&format!("已刪除 {name}（{}）", managed.display()));
    } else if env::legacy_models_dir().join(file).is_file() {
        ui::warn(&format!(
            "{name} 在舊目錄 {}，未自動刪除",
            env::legacy_models_dir().display()
        ));
    } else {
        ui::warn(&format!("{name} 本來就沒下載"));
    }
    Ok(())
}

/// 首次執行的互動選單。
pub fn picker() -> Result<()> {
    if path("turbo").is_some() || path("large-v3").is_some() {
        list();
        return Ok(());
    }
    eprintln!(
        "\n請選擇要下載的模型：\n  1) turbo    (約 1.6G) — 快，日常夠用 ★建議\n  2) large-v3 (約 3.0G) — 最準，較慢\n  3) 兩個都下載\n  4) 稍後再說"
    );
    eprint!("請輸入 [1]：");
    io::stderr().flush().ok();
    let mut ans = String::new();
    io::stdin().read_line(&mut ans).ok();
    match ans.trim() {
        "" | "1" => {
            ensure("turbo", true)?;
            Config::set_model("turbo")?;
        }
        "2" => {
            ensure("large-v3", true)?;
            Config::set_model("large-v3")?;
        }
        "3" => {
            ensure("turbo", true)?;
            ensure("large-v3", true)?;
            Config::set_model("large-v3")?;
        }
        _ => ui::info("稍後可用：t-whisper models pull turbo"),
    }
    Ok(())
}
