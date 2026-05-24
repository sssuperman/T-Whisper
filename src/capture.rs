//! 麥克風擷取（cpal）：列裝置、選裝置、錄音到 16k mono f32。
#![allow(deprecated)] // cpal 0.17 將 DeviceTrait::name 標記 deprecated；name() 仍可用且足夠
use crate::audio;
use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// 列出輸入裝置名稱。
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut v = Vec::new();
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(name) = d.name() {
                v.push(name);
            }
        }
    }
    v
}

/// 系統預設輸入裝置名稱。
pub fn default_input_name() -> Option<String> {
    cpal::default_host()
        .default_input_device()
        .and_then(|d| d.name().ok())
}

/// 依名稱關鍵字（或 default）挑選輸入裝置。
fn pick_device(mic: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if mic.is_empty() || mic == "default" {
        return host.default_input_device().context("找不到預設輸入裝置");
    }
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(name) = d.name()
                && name.contains(mic)
            {
                return Ok(d);
            }
        }
    }
    host.default_input_device()
        .context("找不到符合的輸入裝置，且無預設裝置")
}

/// 共用的擷取緩衝（native rate mono f32，由 cpal callback 持續 append）。
pub type SharedBuf = Arc<Mutex<Vec<f32>>>;

/// 開啟輸入串流，把 mono f32 持續寫入回傳的 buffer。
/// 回傳 (串流, buffer, native 取樣率, 裝置名稱)。串流需由呼叫端保活。
pub fn open_stream(mic: &str) -> Result<(cpal::Stream, SharedBuf, u32, String)> {
    let device = pick_device(mic)?;
    let dev_name = device.name().unwrap_or_else(|_| "未知".into());
    let supported = device
        .default_input_config()
        .context("取得輸入裝置設定失敗（可能是麥克風權限未開）")?;
    let sample_rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let fmt = supported.sample_format();
    let stream_config: cpal::StreamConfig = supported.into();

    let buf: SharedBuf = Arc::new(Mutex::new(Vec::<f32>::new()));
    let err_fn = |e| eprintln!("錄音錯誤: {e}");

    macro_rules! build {
        ($t:ty, $conv:expr) => {{
            let b = buf.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    let mut g = b.lock().unwrap();
                    for frame in data.chunks(channels) {
                        let s: f32 = frame.iter().map(|&x| $conv(x)).sum::<f32>() / channels as f32;
                        g.push(s);
                    }
                },
                err_fn,
                None,
            )
        }};
    }

    let stream = match fmt {
        cpal::SampleFormat::F32 => build!(f32, |x: f32| x),
        cpal::SampleFormat::I16 => build!(i16, |x: i16| x as f32 / 32768.0),
        cpal::SampleFormat::U16 => build!(u16, |x: u16| (x as f32 - 32768.0) / 32768.0),
        other => bail!("不支援的取樣格式：{other:?}"),
    }
    .context("建立錄音串流失敗")?;
    stream.play().context("啟動錄音失敗")?;
    Ok((stream, buf, sample_rate, dev_name))
}

/// 把 16k mono f32 寫成 16-bit PCM WAV。
pub fn write_wav_16k(path: &std::path::Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio::WHISPER_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize()?;
    Ok(())
}
