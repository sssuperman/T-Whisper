//! 即時轉錄：VAD 模式（偵測停頓出整句）與 sliding 模式（單行刷新）。
//! Whisper 是 30 秒批次模型，「即時」靠工程：能量 VAD 切句 + 重複轉錄。
use crate::transcribe::Transcriber;
use crate::{audio, capture, config::Config, trad, ui};
use anyhow::Result;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn run(cfg: &Config, model_path: &Path, model_name: &str, sliding: bool) -> Result<()> {
    crate::transcribe::init_quiet();
    ui::info("載入模型中…");
    let tr = Transcriber::new(model_path, &cfg.lang, cfg.beam)?;

    let stop = Arc::new(AtomicBool::new(false));
    let s2 = stop.clone();
    ctrlc::set_handler(move || s2.store(true, Ordering::Relaxed)).ok();

    let (stream, buf, native_rate, dev) = capture::open_stream(&cfg.mic)?;

    let outdir = std::path::PathBuf::from(&cfg.outdir);
    std::fs::create_dir_all(&outdir).ok();
    let stamp = now_stamp();
    let session_path = outdir.join(format!("{stamp}.txt"));

    ui::info(&format!(
        "模式: {}  |  模型: {}  |  裝置: {}",
        if sliding { "滑動" } else { "VAD" },
        model_name,
        dev
    ));
    ui::info("按 Ctrl+C 結束。開始說話…");
    ui::info("------------------------------------------------------------");

    let session = if sliding {
        run_sliding(&tr, cfg, &buf, native_rate, &stop)?
    } else {
        run_vad(&tr, cfg, &buf, native_rate, &stop)?
    };

    drop(stream);
    if !session.trim().is_empty() {
        std::fs::write(&session_path, session).ok();
        ui::ok(&format!("逐字稿已存：{}", session_path.display()));
    }
    Ok(())
}

/// 取出 buffer 內全部新樣本。
fn drain(buf: &capture::SharedBuf) -> Vec<f32> {
    std::mem::take(&mut *buf.lock().unwrap())
}

fn rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum: f32 = frame.iter().map(|&x| x * x).sum();
    (sum / frame.len() as f32).sqrt()
}

// ---- VAD 模式：偵測停頓出整句（committed lines）----
fn run_vad(
    tr: &Transcriber,
    cfg: &Config,
    buf: &capture::SharedBuf,
    rate: u32,
    stop: &Arc<AtomicBool>,
) -> Result<String> {
    let frame_len = (rate as usize * 30 / 1000).max(1); // 30ms
    let silence_needed_ms = 600usize;
    let max_utter = rate as usize * 20; // 20s 上限
    let min_utter = rate as usize / 4; // 0.25s 下限

    let mut pending: Vec<f32> = Vec::new();
    let mut utter: Vec<f32> = Vec::new();
    let mut in_speech = false;
    let mut silence_ms = 0usize;

    // 自動校準：前 ~0.5s 量環境噪音
    let mut calib: Vec<f32> = Vec::new();
    let mut threshold = 0.012f32;
    let mut calibrated = false;

    let mut session = String::new();

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(30));
        pending.extend(drain(buf));
        while pending.len() >= frame_len {
            let frame: Vec<f32> = pending.drain(..frame_len).collect();

            if !calibrated {
                calib.extend_from_slice(&frame);
                if calib.len() >= rate as usize / 2 {
                    threshold = (rms(&calib) * 3.0).max(0.012);
                    calibrated = true;
                }
                continue;
            }

            let level = rms(&frame);
            if level > threshold {
                in_speech = true;
                silence_ms = 0;
                utter.extend_from_slice(&frame);
            } else if in_speech {
                silence_ms += 30;
                utter.extend_from_slice(&frame);
                if silence_ms >= silence_needed_ms {
                    flush_vad(tr, cfg, rate, &mut utter, &mut session, min_utter);
                    in_speech = false;
                    silence_ms = 0;
                }
            }
            if utter.len() >= max_utter {
                flush_vad(tr, cfg, rate, &mut utter, &mut session, min_utter);
                in_speech = false;
                silence_ms = 0;
            }
        }
    }
    // 收尾：把最後未結束的語句也轉
    flush_vad(tr, cfg, rate, &mut utter, &mut session, min_utter);
    Ok(session)
}

fn flush_vad(
    tr: &Transcriber,
    cfg: &Config,
    rate: u32,
    utter: &mut Vec<f32>,
    session: &mut String,
    min_utter: usize,
) {
    if utter.len() < min_utter {
        utter.clear();
        return;
    }
    let audio16 = audio::resample_to_16k(utter, rate);
    utter.clear();
    if let Ok(segs) = tr.run(&audio16) {
        for s in segs {
            let text = trad::to_traditional(&s.text, cfg.to_traditional);
            if text.is_empty() {
                continue;
            }
            println!("{text}");
            session.push_str(&text);
            session.push('\n');
        }
        let _ = std::io::stdout().flush();
    }
}

// ---- 滑動模式：單行刷新顯示部分結果，停頓時固定成歷史 ----
fn run_sliding(
    tr: &Transcriber,
    cfg: &Config,
    buf: &capture::SharedBuf,
    rate: u32,
    stop: &Arc<AtomicBool>,
) -> Result<String> {
    let window_max = rate as usize * 8; // 視窗最長 8s
    let frame_len = (rate as usize * 30 / 1000).max(1);
    let mut window: Vec<f32> = Vec::new();
    let mut pending: Vec<f32> = Vec::new();
    let mut silence_ms = 0usize;
    let mut threshold = 0.012f32;
    let mut calib: Vec<f32> = Vec::new();
    let mut calibrated = false;
    let mut session = String::new();
    let mut last_line = String::new();

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(120));
        pending.extend(drain(buf));
        let mut had_speech = false;
        while pending.len() >= frame_len {
            let frame: Vec<f32> = pending.drain(..frame_len).collect();
            if !calibrated {
                calib.extend_from_slice(&frame);
                if calib.len() >= rate as usize / 2 {
                    threshold = (rms(&calib) * 3.0).max(0.012);
                    calibrated = true;
                }
                continue;
            }
            let level = rms(&frame);
            window.extend_from_slice(&frame);
            if level > threshold {
                had_speech = true;
                silence_ms = 0;
            } else {
                silence_ms += 30;
            }
        }
        if window.len() > window_max {
            let cut = window.len() - window_max;
            window.drain(..cut);
        }
        if !calibrated || window.is_empty() {
            continue;
        }

        // 停頓夠久 → 固定當前行、清窗
        if silence_ms >= 800 && !last_line.is_empty() {
            commit_line(&last_line, &mut session);
            last_line.clear();
            window.clear();
            continue;
        }
        // 有語音才重轉（避免靜音時空轉）
        if had_speech || !last_line.is_empty() {
            let audio16 = audio::resample_to_16k(&window, rate);
            if let Ok(segs) = tr.run(&audio16) {
                let joined: String = segs
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                let text = trad::to_traditional(joined.trim(), cfg.to_traditional);
                if !text.is_empty() {
                    render_partial(&text);
                    last_line = text;
                }
            }
        }
    }
    if !last_line.is_empty() {
        commit_line(&last_line, &mut session);
    }
    Ok(session)
}

fn commit_line(text: &str, session: &mut String) {
    print!("\r\x1b[2K{text}\n");
    let _ = std::io::stdout().flush();
    session.push_str(text);
    session.push('\n');
}

fn render_partial(text: &str) {
    let w = term_width().saturating_sub(1).max(10);
    let shown = truncate_tail(text, w);
    print!("\r\x1b[2K{shown}");
    let _ = std::io::stdout().flush();
}

fn now_stamp() -> String {
    std::process::Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "live".into())
}

fn term_width() -> usize {
    if let Ok(c) = std::env::var("COLUMNS")
        && let Ok(n) = c.parse::<usize>()
        && n > 0
    {
        return n;
    }
    if let Ok(out) = std::process::Command::new("tput").arg("cols").output()
        && let Ok(n) = String::from_utf8_lossy(&out.stdout).trim().parse::<usize>()
        && n > 0
    {
        return n;
    }
    100
}

fn char_width(c: char) -> usize {
    let o = c as u32;
    let wide = (0x1100..=0x115F).contains(&o)
        || (0x2E80..=0xA4CF).contains(&o)
        || (0xAC00..=0xD7A3).contains(&o)
        || (0xF900..=0xFAFF).contains(&o)
        || (0xFE30..=0xFE4F).contains(&o)
        || (0xFF00..=0xFF60).contains(&o)
        || (0xFFE0..=0xFFE6).contains(&o);
    if wide { 2 } else { 1 }
}

/// 保留尾端、截到顯示寬度 maxw（讓最新文字可見）。
fn truncate_tail(s: &str, maxw: usize) -> String {
    let total: usize = s.chars().map(char_width).sum();
    if total <= maxw {
        return s.to_string();
    }
    let mut acc = 0;
    let mut kept: Vec<char> = Vec::new();
    for c in s.chars().rev() {
        let cw = char_width(c);
        if acc + cw > maxw {
            break;
        }
        acc += cw;
        kept.push(c);
    }
    kept.into_iter().rev().collect()
}
