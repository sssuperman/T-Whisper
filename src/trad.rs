//! 簡轉繁（OpenCC s2twp）。目前透過 opencc 子程序；之後可評估純 Rust。
use crate::env;
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// 把字串簡轉繁；opencc 不在或關閉時回傳原字串。
pub fn to_traditional(s: &str, enabled: bool) -> String {
    if !enabled || !env::has_cmd("opencc") {
        return s.to_string();
    }
    convert(s).unwrap_or_else(|_| s.to_string())
}

fn convert(s: &str) -> Result<String> {
    let mut child = Command::new("opencc")
        .args(["-c", "s2twp.json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("啟動 opencc 失敗")?;
    child
        .stdin
        .take()
        .context("opencc stdin")?
        .write_all(s.as_bytes())?;
    let out = child.wait_with_output()?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string())
}

/// 轉換一個檔案或資料夾（給 trad 子指令用）。
pub fn convert_path(target: &str, out_override: Option<&str>) -> Result<()> {
    use std::path::Path;
    env::has_cmd("opencc")
        .then_some(())
        .context("需要 opencc：brew install opencc")?;
    let p = Path::new(target);
    if p.is_dir() {
        for entry in std::fs::read_dir(p)? {
            let f = entry?.path();
            let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "txt" | "srt" | "vtt") {
                continue;
            }
            let name = f.to_string_lossy();
            if name.contains(".trad.") {
                continue;
            }
            let out = f.with_extension(format!("trad.{ext}"));
            convert_file(&f.to_string_lossy(), &out.to_string_lossy())?;
        }
    } else if p.is_file() {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        let out = out_override.map(String::from).unwrap_or_else(|| {
            p.with_extension(format!("trad.{ext}"))
                .to_string_lossy()
                .into_owned()
        });
        convert_file(target, &out)?;
    } else {
        anyhow::bail!("找不到：{target}");
    }
    Ok(())
}

fn convert_file(input: &str, output: &str) -> Result<()> {
    let status = Command::new("opencc")
        .args(["-c", "s2twp.json", "-i", input, "-o", output])
        .status()?;
    if status.success() {
        crate::ui::info(&format!("繁體：{output}"));
    }
    Ok(())
}
