//! 設定檔 ~/.config/t-whisper/config（key=value）。
use crate::{env, ui};
use anyhow::Result;
use std::fs;

#[derive(Debug, Clone)]
pub struct Config {
    pub lang: String,
    pub model: String,
    pub beam: i32,
    pub mic: String,
    pub to_traditional: bool,
    pub outdir: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lang: "zh".into(),
            model: "turbo".into(),
            beam: 5,
            mic: "default".into(),
            to_traditional: true,
            outdir: env::default_outdir().to_string_lossy().into_owned(),
        }
    }
}

impl Config {
    /// 從設定檔載入（缺檔則用預設）。
    pub fn load() -> Self {
        let mut c = Config::default();
        let p = env::config_path();
        let Ok(text) = fs::read_to_string(&p) else {
            return c;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim();
            // 去除行內註解與空白
            let v = v.split('#').next().unwrap_or("").trim();
            match k {
                "lang" => c.lang = v.into(),
                "model" => c.model = v.into(),
                "beam" => {
                    if let Ok(n) = v.parse() {
                        c.beam = n;
                    }
                }
                "mic" => c.mic = v.into(),
                "to_traditional" => c.to_traditional = v == "1" || v.eq_ignore_ascii_case("true"),
                "outdir" => {
                    c.outdir = if let Some(rest) = v.strip_prefix("~/") {
                        env::home().join(rest).to_string_lossy().into_owned()
                    } else {
                        v.into()
                    }
                }
                _ => {}
            }
        }
        c
    }

    /// 若設定檔不存在則寫入預設範本。
    pub fn write_default_if_absent() -> Result<()> {
        let p = env::config_path();
        if p.exists() {
            return Ok(());
        }
        fs::create_dir_all(env::config_dir())?;
        let body = "\
# T-Whisper 設定（CLI 旗標可覆蓋這裡）
lang=zh            # 轉錄語言（zh / en / ja …）
model=turbo        # 預設模型：turbo（快）或 large-v3（準）
beam=5             # beam search 寬度，越大越準越慢
mic=default        # 麥克風名稱關鍵字（default=系統預設輸入）
to_traditional=1   # 1=輸出簡轉繁（台灣用語），0=不轉
outdir=~/whisper-transcripts
";
        fs::write(&p, body)?;
        ui::ok(&format!("已建立設定檔：{}", p.display()));
        Ok(())
    }

    /// 更新設定檔中的 model=（picker 用）。
    pub fn set_model(model: &str) -> Result<()> {
        Self::write_default_if_absent()?;
        let p = env::config_path();
        let text = fs::read_to_string(&p)?;
        let mut found = false;
        let mut out = String::new();
        for line in text.lines() {
            if line.trim_start().starts_with("model=") {
                out.push_str(&format!("model={model}\n"));
                found = true;
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        if !found {
            out.push_str(&format!("model={model}\n"));
        }
        fs::write(&p, out)?;
        Ok(())
    }
}
