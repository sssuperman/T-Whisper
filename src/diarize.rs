//! 說話者分離（diarization）：sherpa-onnx（pyannote 分段 + CAM++ 中文聲紋）。
//! 與 whisper 的轉錄 segment 用「時間重疊」合併，標出 [說話者 N]。
use crate::{env, transcribe::Segment, ui};
use anyhow::Result;
use std::path::PathBuf;

pub struct Turn {
    pub start_ms: i64,
    pub end_ms: i64,
    pub speaker: i32,
}

fn model_paths() -> (PathBuf, PathBuf) {
    let d = env::models_dir().join("diarize");
    (d.join("segmentation.onnx"), d.join("embedding.onnx"))
}

/// 對 16k mono 音訊做說話者分離。num_speakers=None 表示自動估算人數。
pub fn diarize(samples_16k: &[f32], num_speakers: Option<i32>) -> Result<Vec<Turn>> {
    let (seg, emb) = model_paths();
    if !seg.is_file() || !emb.is_file() {
        anyhow::bail!(
            "缺少 diarization 模型（放到 {}）",
            env::models_dir().join("diarize").display()
        );
    }

    let cfg = sherpa_rs::diarize::DiarizeConfig {
        // num_clusters <=0 → 用 threshold 自動估算人數
        num_clusters: Some(num_speakers.unwrap_or(-1)),
        threshold: Some(0.5),
        min_duration_on: Some(0.3),
        min_duration_off: Some(0.5),
        provider: None,
        debug: false,
    };

    let mut d = sherpa_rs::diarize::Diarize::new(&seg, &emb, cfg)
        .map_err(|e| anyhow::anyhow!("初始化 diarization 失敗：{e}"))?;
    let segs = d
        .compute(samples_16k.to_vec(), None)
        .map_err(|e| anyhow::anyhow!("diarization 計算失敗：{e}"))?;

    Ok(segs
        .into_iter()
        .map(|s| Turn {
            start_ms: (s.start * 1000.0) as i64,
            end_ms: (s.end * 1000.0) as i64,
            speaker: s.speaker,
        })
        .collect())
}

/// 把每個 whisper segment 指派給時間重疊最大的說話者。回傳 speaker 編號（找不到回 -1）。
fn speaker_for(seg: &Segment, turns: &[Turn]) -> i32 {
    let mut best = -1i32;
    let mut best_ov = 0i64;
    for t in turns {
        let ov = (seg.end_ms.min(t.end_ms) - seg.start_ms.max(t.start_ms)).max(0);
        if ov > best_ov {
            best_ov = ov;
            best = t.speaker;
        }
    }
    best
}

/// 合併 whisper segment 與 diarization turn → 每行附 [說話者 N]，連續同人合併。
/// 回傳 (印給終端的字串, 存檔字串)；text 已是繁體。
pub fn merge_lines(
    segs: &[Segment],
    turns: &[Turn],
    text_of: impl Fn(&Segment) -> String,
) -> String {
    let n_speakers = turns.iter().map(|t| t.speaker).max().map(|m| m + 1).unwrap_or(0);
    ui::info(&format!("偵測到 {n_speakers} 位說話者"));

    let mut out = String::new();
    let mut last_spk = i32::MIN;
    for s in segs {
        let spk = speaker_for(s, turns);
        let text = text_of(s);
        if text.is_empty() {
            continue;
        }
        if spk != last_spk {
            out.push_str(&format!("\n[說話者 {}]\n", spk + 1));
            last_spk = spk;
        }
        out.push_str(&format!(
            "[{} --> {}]  {}\n",
            crate::transcribe::fmt_ts(s.start_ms),
            crate::transcribe::fmt_ts(s.end_ms),
            text
        ));
    }
    out
}
