# T-Whisper 架構修訂：Rust + whisper-rs

日期：2026-05-23
取代先前的 shell 版（保留於 `legacy-shell/`，功能已完成可用，作為參考）。

## 為什麼從 shell 換成編譯語言

shell 版雖然可用，但連續踩到 shell 特有地雷（locale 相關變數解析、BSD vs gawk、`local`+`trap`+`set -u` 作用域、`pipefail`+`grep -q` 的 SIGPIPE），符合 systematic-debugging 的「修一個又冒一個 → 該換架構」訊號。改用有型別的編譯語言根除這些問題，並換得單一執行檔分發。

## 為什麼是 Rust + whisper-rs（而非 Go / Swift / sherpa）

選型關鍵約束：**主要使用者（含學長）有 Intel Mac** + 要 Whisper 等級繁體（已驗證 large-v3 + opencc）+ 即時與批次 + 單一執行檔 + 好除錯。

| 方案 | 否決原因 |
|---|---|
| Go + whisper.cpp bindings | bindings 只批次、cgo 抵銷交叉編譯優勢，是最弱解 |
| Swift + WhisperKit | **Intel Mac 出局**（CoreML/ANE 僅 Apple Silicon）；學長是 Intel |
| Rust + sherpa-onnx | 真串流、維護健康，但換掉 Whisper 模型，繁體需重新 POC |
| **Rust + whisper-rs** ✅ | whisper.cpp 編入單檔、Metal(AS)/CPU(Intel)、universal binary、沿用已驗證的 Whisper 繁體管線 |

whisper-rs 維護備註：GitHub repo 已封存（2025-07-30），開發**搬至 Codeberg**（仍活躍），crates.io 持續發佈（0.16.0）。它是 whisper.cpp 上的薄 FFI；風險用「鎖版本 + 必要時自維護 FFI」控制，核心 whisper.cpp（ggml-org）極活躍。

## 技術堆疊

- **語言**：Rust（stable 1.95）
- **推論**：`whisper-rs` 0.16（features: `metal`；Intel slice 不開 metal 走 CPU）
- **麥克風擷取**：`cpal`（純 Rust，CoreAudio）
- **音檔解碼**（file 模式，取代 ffmpeg）：`symphonia`（純 Rust，mp3/aac/mp4/wav/flac…）；重取樣 `rubato` 或自寫線性重取樣到 16k mono
- **VAD（即時切句）**：`voice_activity_detector`（Silero V5）或 `voice-stream`
- **CLI**：`clap`（derive，子指令 + --help + --version）
- **簡轉繁**：先 shell out 到 `opencc -c s2twp.json`（小依賴）；之後評估純 Rust OpenCC
- **分發**：`cargo build --release` 兩個 target（arm64 + x86_64）→ `lipo` 合併 universal binary；GitHub Releases + 一行 install.sh 下載對應/通用執行檔

## 指令介面（沿用先前設計）

```
t-whisper live [--sliding]   即時轉錄（VAD / 滑動）
t-whisper rec                錄音 → 整段精準轉錄（絕對時間戳、零重複）
t-whisper file <檔案>        轉現成音檔／影片
t-whisper trad <檔案|資料夾> 簡轉繁
t-whisper models [list|pull|rm]
t-whisper mics / doctor / update / --help / --version
```

設定檔 `~/.config/t-whisper/config`（lang/model/beam/mic/to_traditional），模型存 `~/.local/share/t-whisper/models/`，沿用既有 `~/models/whisper/*.bin`。

## 即時串流策略

Whisper 是 30 秒批次模型，「即時」靠工程：cpal 持續擷取 → Silero VAD 偵測語句邊界 → 切 chunk 跑 `state.full()` → new-segment callback 邊出字。滑動模式用重疊視窗 + 前綴穩定化。參考實作：`operator-kit/whisper-cpp-plus-rs`、`voice-stream`。

## 後續探索計畫（roadmap — 使用者有興趣，暫不納入本期）

1. **Rust + sherpa-onnx**：模型原生**真低延遲串流**、跨架構、無 Python。需先 POC 驗證繁體中文準確度（Zipformer/Paraformer 中文模型）。若 whisper-rs 的即時延遲不夠，這是升級路線。
2. **Swift + WhisperKit**：Apple Silicon 專屬最佳化版本（ANE、最低記憶體、內建 `AudioStreamTranscriber` 即時）。可作為「Apple Silicon 使用者的高效版」分支，但**不支援 Intel**，故與本期 universal 目標並行而非取代。

## 驗證里程碑

1. ✅ 工具鏈：Rust 1.95 + arm64/x86_64 target + cmake + Xcode CLT
2. ✅ 最小驗證：whisper-rs 編譯（含 whisper.cpp）+ Metal + 轉錄 test/sample.wav
3. ✅ file / trad / doctor / models / config（批次管線，純 Rust 解碼 symphonia）
4. ✅ rec（cpal 錄音 + Ctrl+C + 整段轉錄 + 存 WAV/TXT）+ mics
5. ☐ live（cpal + VAD + 滑動渲染）— 唯一未完成
6. ✅ universal binary（lipo arm64+x86_64）+ install.sh + uninstall.sh + README + Makefile + GitHub Actions release workflow

## 現況（2026-05-23）

可用指令：file / rec / mics / trad / models / doctor / --version。
單一執行檔，whisper.cpp 編入，Metal(AS)/CPU(Intel)，繁體（opencc）。唯一外部依賴 opencc。
分發：`make universal` 產 universal binary；推 git tag `v*` 觸發 CI 發 release；學長用 `curl … install.sh | bash` 一行安裝。
**待辦**：`live` 即時模式（cpal 連續擷取 + Silero VAD 切句 + 滑動視窗 + 單行渲染）。
