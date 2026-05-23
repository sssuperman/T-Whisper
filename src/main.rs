//! T-Whisper — 本機中文語音轉錄（whisper.cpp via whisper-rs，繁體輸出）。
mod audio;
mod capture;
mod config;
mod diarize;
mod env;
mod live;
mod models;
mod trad;
mod transcribe;
mod ui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use transcribe::Segment;

#[derive(Parser)]
#[command(
    name = "t-whisper",
    version,
    about = "本機中文語音轉錄（離線、繁體、whisper.cpp）"
)]
struct Cli {
    /// 模型：turbo / large-v3 / 或 .bin 路徑（覆蓋設定檔）
    #[arg(long, global = true)]
    model: Option<String>,
    /// 語言（預設 zh）
    #[arg(long, global = true)]
    lang: Option<String>,
    /// beam search 寬度
    #[arg(long, global = true)]
    beam: Option<i32>,
    /// 麥克風名稱關鍵字
    #[arg(long, global = true)]
    mic: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 即時轉錄（預設 VAD；--sliding 邊講邊吐）
    Live {
        #[arg(long)]
        sliding: bool,
    },
    /// 錄音 → 結束後整段精準轉錄（錄音中顯示即時粗略預覽）
    Rec {
        /// 標出說話者（diarization）
        #[arg(long)]
        diarize: bool,
        /// 指定說話者人數（不給則自動估算）
        #[arg(long)]
        speakers: Option<i32>,
        /// 自動估算時的合併門檻（越高越願意合併、群越少；預設 0.7）
        #[arg(long)]
        threshold: Option<f32>,
        /// 關閉錄音中的即時預覽
        #[arg(long)]
        no_preview: bool,
    },
    /// 轉現成音檔／影片
    File {
        path: String,
        /// 標出說話者（diarization）
        #[arg(long)]
        diarize: bool,
        /// 指定說話者人數（不給則自動估算）
        #[arg(long)]
        speakers: Option<i32>,
        /// 自動估算時的合併門檻（越高越願意合併、群越少；預設 0.7）
        #[arg(long)]
        threshold: Option<f32>,
    },
    /// 簡轉繁（檔案／資料夾／stdin）
    Trad {
        target: Option<String>,
        out: Option<String>,
    },
    /// 模型管理
    Models {
        #[command(subcommand)]
        action: Option<ModelsAction>,
    },
    /// 列出麥克風裝置
    Mics,
    /// 檢查依賴／模型／環境
    Doctor,
    /// 更新到最新版
    Update,
}

#[derive(Subcommand)]
enum ModelsAction {
    List,
    Pull {
        name: String,
    },
    Rm {
        name: String,
    },
    /// 首次執行的互動選單
    Picker,
}

fn main() {
    let cli = Cli::parse();
    let mut cfg = Config::load();
    if let Some(v) = cli.lang.clone() {
        cfg.lang = v;
    }
    if let Some(v) = cli.beam {
        cfg.beam = v;
    }
    if let Some(v) = cli.mic.clone() {
        cfg.mic = v;
    }
    let model_override = cli.model.clone();

    let res = match cli.cmd {
        Cmd::File {
            ref path,
            diarize,
            speakers,
            threshold,
        } => cmd_file(path, &cfg, model_override.as_deref(), diarize, speakers, threshold),
        Cmd::Trad {
            ref target,
            ref out,
        } => cmd_trad(target.as_deref(), out.as_deref()),
        Cmd::Models { ref action } => cmd_models(action),
        Cmd::Doctor => cmd_doctor(&cfg),
        Cmd::Mics => cmd_mics(),
        Cmd::Rec {
            diarize,
            speakers,
            threshold,
            no_preview,
        } => cmd_rec(
            &cfg,
            model_override.as_deref(),
            diarize,
            speakers,
            threshold,
            no_preview,
        ),
        Cmd::Live { sliding } => cmd_live(&cfg, model_override.as_deref(), sliding),
        Cmd::Update => {
            ui::info("update：請重跑安裝指令或 git pull（自動更新建置中）");
            Ok(())
        }
    };

    if let Err(e) = res {
        ui::error(&format!("{e}"));
        // anyhow 的 context 鏈
        for cause in e.chain().skip(1) {
            ui::fix(&format!("{cause}"));
        }
        std::process::exit(1);
    }
}

fn cmd_file(
    path: &str,
    cfg: &Config,
    model_override: Option<&str>,
    diarize: bool,
    speakers: Option<i32>,
    threshold: Option<f32>,
) -> Result<()> {
    if !std::path::Path::new(path).is_file() {
        anyhow::bail!("找不到檔案：{path}");
    }
    let model_name = model_override.unwrap_or(&cfg.model);
    let model = models::resolve(model_name, false)?;

    transcribe::init_quiet();
    ui::info("前處理音訊（→ 16k mono）…");
    let samples = audio::decode_to_whisper(path)?;
    ui::info(&format!("轉錄中（{} + beam {}）…", model_name, cfg.beam));

    let segs = transcribe::transcribe_file(&model, &samples, &cfg.lang, cfg.beam)?;
    let out_path = Path::new(path).with_extension("txt");
    emit_result(&segs, &samples, cfg, &out_path, diarize, speakers, threshold)
}

/// 輸出逐字稿：開 diarize 時附 [說話者 N]，否則時間戳逐行。file / rec 共用。
fn emit_result(
    segs: &[Segment],
    samples: &[f32],
    cfg: &Config,
    out_path: &Path,
    diarize: bool,
    speakers: Option<i32>,
    threshold: Option<f32>,
) -> Result<()> {
    if diarize {
        ui::info("說話者分離中…");
        let turns = diarize::diarize(samples, speakers, threshold.unwrap_or(0.7))?;
        let to_trad = cfg.to_traditional;
        let body = diarize::merge_lines(segs, &turns, |s| trad::to_traditional(&s.text, to_trad));
        print!("{body}");
        std::fs::write(out_path, &body).context("寫入逐字稿失敗")?;
        ui::ok(&format!("逐字稿（含說話者）：{}", out_path.display()));
        Ok(())
    } else {
        emit_segments(segs, cfg, out_path)
    }
}

/// 印出 + 存檔（繁體、絕對時間戳）；file / rec 共用。
fn emit_segments(segs: &[Segment], cfg: &Config, out_path: &Path) -> Result<()> {
    let mut body = String::new();
    println!();
    for s in segs {
        let text = trad::to_traditional(&s.text, cfg.to_traditional);
        let line = format!(
            "[{} --> {}]  {}",
            transcribe::fmt_ts(s.start_ms),
            transcribe::fmt_ts(s.end_ms),
            text
        );
        println!("{line}");
        body.push_str(&line);
        body.push('\n');
    }
    std::fs::write(out_path, body).context("寫入逐字稿失敗")?;
    ui::ok(&format!("逐字稿：{}", out_path.display()));
    Ok(())
}

fn timestamp() -> String {
    std::process::Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "recording".into())
}

fn cmd_live(cfg: &Config, model_override: Option<&str>, sliding: bool) -> Result<()> {
    // sliding 求即時 → 預設 turbo；VAD 用設定檔模型（可較準）
    let model_name = if sliding {
        model_override.unwrap_or("turbo")
    } else {
        model_override.unwrap_or(&cfg.model)
    };
    let model = models::resolve(model_name, false)?;
    live::run(cfg, &model, model_name, sliding)
}

fn cmd_mics() -> Result<()> {
    ui::info("可用麥克風（輸入裝置）：");
    for (i, name) in capture::list_input_devices().iter().enumerate() {
        eprintln!("  [{i}] {name}");
    }
    if let Some(d) = capture::default_input_name() {
        ui::info(&format!("系統預設輸入：{d}"));
    }
    Ok(())
}

fn cmd_rec(
    cfg: &Config,
    model_override: Option<&str>,
    diarize: bool,
    speakers: Option<i32>,
    threshold: Option<f32>,
    no_preview: bool,
) -> Result<()> {
    let model_name = model_override.unwrap_or(&cfg.model);
    let model = models::resolve(model_name, false)?; // 結束後精準轉錄用
    transcribe::init_quiet();

    let stop = Arc::new(AtomicBool::new(false));
    let s2 = stop.clone();
    ctrlc::set_handler(move || s2.store(true, Ordering::Relaxed)).ok();

    let (stream, buf, native_rate, dev) = capture::open_stream(&cfg.mic)?;
    ui::info(&format!("● 錄音中（裝置：{dev}）…按 Ctrl+C 停止並精準轉錄"));

    if no_preview {
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }
    } else {
        ui::info("（下方為即時粗略預覽，最終以結束後的精準轉錄為準）");
        ui::info("------------------------------------------------------------");
        // 預覽用 turbo（快、beam 1）；沒下載就退回主模型
        let preview_path = models::path("turbo").unwrap_or_else(|| model.clone());
        match transcribe::Transcriber::new(&preview_path, &cfg.lang, 1) {
            Ok(ptr) => live::preview_loop(&ptr, cfg, &buf, native_rate, &stop),
            Err(_) => {
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
    drop(stream);

    let full = std::mem::take(&mut *buf.lock().unwrap());
    if full.is_empty() {
        ui::warn("沒有錄到音訊（請到 系統設定 → 隱私權與安全性 → 麥克風 開啟權限）");
        return Ok(());
    }
    let samples = audio::resample_to_16k(&full, native_rate);
    ui::info(&format!("錄音長度約 {} 秒", samples.len() / 16000));

    let outdir = PathBuf::from(&cfg.outdir);
    std::fs::create_dir_all(&outdir)?;
    let stamp = timestamp();
    let wav = outdir.join(format!("{stamp}.wav"));
    capture::write_wav_16k(&wav, &samples)?;

    ui::info(&format!("轉錄中（{model_name} + beam {}）…", cfg.beam));
    let segs = transcribe::transcribe_file(&model, &samples, &cfg.lang, cfg.beam)?;
    emit_result(
        &segs,
        &samples,
        cfg,
        &outdir.join(format!("{stamp}.txt")),
        diarize,
        speakers,
        threshold,
    )?;
    ui::ok(&format!("原始錄音：{}", wav.display()));
    Ok(())
}

fn cmd_trad(target: Option<&str>, out: Option<&str>) -> Result<()> {
    match target {
        None => {
            // stdin 模式
            if std::io::stdin().is_terminal() {
                anyhow::bail!("請指定檔案或資料夾，或用管線餵入文字");
            }
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            print!("{}", trad::to_traditional(&input, true));
            Ok(())
        }
        Some(t) => trad::convert_path(t, out),
    }
}

fn cmd_models(action: &Option<ModelsAction>) -> Result<()> {
    match action {
        None | Some(ModelsAction::List) => {
            models::list();
            Ok(())
        }
        Some(ModelsAction::Pull { name }) => {
            models::ensure(name, true)?;
            models::list();
            Ok(())
        }
        Some(ModelsAction::Rm { name }) => models::remove(name),
        Some(ModelsAction::Picker) => models::picker(),
    }
}

fn cmd_doctor(_cfg: &Config) -> Result<()> {
    ui::info("T-Whisper 環境檢查");
    ui::info("------------------------------------------------------------");
    let mut problems = 0;

    match env::brew_prefix() {
        Some(p) => ui::ok(&format!("Homebrew（{p}）")),
        None => {
            ui::warn("找不到 Homebrew");
            problems += 1;
        }
    }
    // opencc 為繁體所需；whisper.cpp 已編入 binary，不再依賴外部 whisper-cli
    if env::has_cmd("opencc") {
        ui::ok("opencc（簡轉繁）");
    } else {
        ui::warn("缺 opencc → brew install opencc（繁體輸出需要）");
        problems += 1;
    }

    let mut has_model = false;
    for name in ["turbo", "large-v3"] {
        if let Some(p) = models::path(name) {
            ui::ok(&format!("模型 {name}（{}）", p.display()));
            has_model = true;
        }
    }
    if !has_model {
        ui::warn("尚無模型 → t-whisper models pull turbo");
        problems += 1;
    }

    let mics = capture::list_input_devices();
    if mics.is_empty() {
        ui::warn("列不到麥克風 → 系統設定 → 隱私權與安全性 → 麥克風，開啟終端機權限");
        problems += 1;
    } else {
        ui::ok(&format!("麥克風可列出（{} 個輸入裝置）", mics.len()));
    }

    ui::info("------------------------------------------------------------");
    if problems == 0 {
        ui::ok("一切就緒！試試：t-whisper file <音檔>");
    } else {
        ui::warn(&format!("有 {problems} 項待處理（見上方）"));
    }
    Ok(())
}
