# T-Whisper

本機（離線）中文語音轉錄命令列工具。以 [whisper.cpp](https://github.com/ggml-org/whisper.cpp)（透過
[whisper-rs](https://codeberg.org/tazz4843/whisper-rs)）編成**單一執行檔**，輸出**台灣繁體**（OpenCC s2twp）。
無 GUI、無雲端、隱私不外流。

> macOS，Apple Silicon 與 Intel 通用（universal binary）。Apple Silicon 走 Metal GPU 加速。信誌科技出品，MIT 授權。

## 安裝

```bash
curl -fsSL https://raw.githubusercontent.com/sssuperman/T-Whisper/main/install.sh | bash
```

whisper.cpp 已**編入執行檔**，安裝時不需裝 whisper-cpp；只會裝 `opencc`（繁體輸出用）。
裝完先檢查：

```bash
t-whisper doctor
```

> 若提示 `~/.local/bin 不在 PATH`，在 `~/.zshrc` 末端加一行後重開終端機：
> `export PATH="$HOME/.local/bin:$PATH"`

## 用法

```bash
t-whisper rec                 # 開會／演講：錄音 → Ctrl+C 後出乾淨逐字稿（最推薦）
t-whisper file 演講.mp4       # 轉現成音檔／影片（mp3/mp4/m4a/wav…，純 Rust 解碼）
t-whisper trad 字幕.srt       # 簡體字幕轉台灣繁體
t-whisper mics                # 列出麥克風裝置
t-whisper models list         # 看已下載哪些模型
t-whisper live                # 即時轉錄（VAD：偵測停頓出整句，較準）
t-whisper live --sliding      # 即時轉錄（單行刷新，邊講邊出；用 turbo 求即時）
```

逐字稿預設存到 `~/whisper-transcripts/`，含**絕對時間戳、繁體**。

## 設定

`~/.config/t-whisper/config`（CLI 旗標可臨時覆蓋）：

```
lang=zh            # 語言
model=turbo        # turbo（快）或 large-v3（準）
beam=5             # beam search 寬度
mic=default        # 麥克風名稱關鍵字（default=系統預設）
to_traditional=1   # 1=簡轉繁，0=不轉
outdir=~/whisper-transcripts
```

臨時覆蓋：`t-whisper file 講稿.wav --model large-v3`、`t-whisper rec --mic MacBook`。

## 模型

| 名稱 | 大小 | 說明 |
|---|---|---|
| `turbo` | ~1.6G | 快，日常夠用（預設） |
| `large-v3` | ~3.0G | 最準，較慢 |

`t-whisper models pull large-v3` 下載、`t-whisper models rm large-v3` 刪除。
模型存 `~/.local/share/t-whisper/models/`，也會沿用既有 `~/models/whisper/*.bin`。

## 常見問題

- **沒錄到音／沒逐字稿**：系統設定 → 隱私權與安全性 → 麥克風，開啟終端機權限。
- **想轉系統音訊（Teams／影片內部聲音）**：裝 BlackHole 虛擬聲卡把系統音導入，再 `t-whisper rec --mic BlackHole`。
- **簡繁**：whisper 中文簡繁不定，本工具預設用 OpenCC s2twp 自動轉台灣繁體；關閉設 `to_traditional=0`。
- **Intel Mac**：可用（CPU 推論，較 Apple Silicon 慢）。

## 從原始碼編譯

需要 Rust（stable）、Xcode Command Line Tools、cmake。

```bash
make build       # 本機架構
make universal   # arm64 + x86_64 → target/universal/t-whisper（lipo 合併）
make test        # 煙霧測試
make install     # 安裝 universal binary 到 ~/.local/bin
```

## 解除安裝

```bash
bash ~/.local/share/t-whisper/uninstall.sh   # 或 repo 內 ./uninstall.sh
```

## 技術

- 推論：whisper-rs（whisper.cpp）；Apple Silicon Metal、Intel CPU
- 錄音：cpal（純 Rust）；音檔解碼：symphonia（純 Rust，免 ffmpeg）
- 繁體：OpenCC s2twp
- 唯一外部依賴：`opencc`
