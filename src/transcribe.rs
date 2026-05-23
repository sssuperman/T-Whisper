//! whisper-rs 推論核心：16k mono f32 → 帶時間戳的 segment。
use crate::env;
use anyhow::{Context, Result};
use std::sync::OnceLock;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

/// 載入模型並轉錄整段音訊（批次）。
pub fn transcribe_file(
    model_path: &std::path::Path,
    audio: &[f32],
    lang: &str,
    beam: i32,
) -> Result<Vec<Segment>> {
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .context("載入模型失敗")?;
    let mut state = ctx.create_state().context("建立 whisper state 失敗")?;

    let mut params = FullParams::new(SamplingStrategy::BeamSearch {
        beam_size: beam.max(1),
        patience: -1.0,
    });
    params.set_language(Some(lang));
    params.set_n_threads(env::threads());
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);

    state.full(params, audio).context("轉錄失敗")?;

    let n = state.full_n_segments();
    let mut segs = Vec::with_capacity(n as usize);
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            let text = seg.to_str_lossy().unwrap_or_default().trim().to_string();
            // whisper 時間戳單位為 10ms
            segs.push(Segment {
                start_ms: seg.start_timestamp() * 10,
                end_ms: seg.end_timestamp() * 10,
                text,
            });
        }
    }
    Ok(segs)
}

/// 抑制 whisper.cpp 自身的 log（只在第一次呼叫設定）。
pub fn init_quiet() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        whisper_rs::install_logging_hooks();
    });
}

/// [HH:MM:SS.mmm] 格式化。
pub fn fmt_ts(ms: i64) -> String {
    let total = ms.max(0);
    let h = total / 3_600_000;
    let m = (total % 3_600_000) / 60_000;
    let s = (total % 60_000) / 1000;
    let mi = total % 1000;
    format!("{h:02}:{m:02}:{s:02}.{mi:03}")
}
