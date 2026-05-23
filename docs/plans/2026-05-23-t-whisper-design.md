# T-Whisper 設計文件

日期：2026-05-23
作者：信誌科技

## 目標

把現有 4 個本機 whisper 轉錄腳本（`whisper-live` / `whisper-rec` / `whisper-render` / `to-trad`）
整理成 production-ready、可分享給他人（含學長）的命令列工具。無 GUI。

## 需求（brainstorming 結論）

- **目標平台**：macOS 為主，需同時支援 Apple Silicon（`/opt/homebrew`）與 Intel（`/usr/local`）。
- **安裝體驗**：會用終端機但要極簡 → 一行 `curl … | bash` 自動裝依賴、下載模型、安裝指令。
- **托管**：公開 GitHub repo（`T-Whisper`）。
- **功能優化**：友善依賴／錯誤訊息、模型首次執行自動下載、統一 `t-whisper` 主指令 + `--help`、設定檔 + 麥克風選擇。
- **品牌**：`T-Whisper`（對齊 T-Patrol / T-Vouch）。

## 命名與大小寫

- canonical 指令 = 小寫 `t-whisper`（CLI 慣例、免按 Shift）。
- 安裝時額外建真正的 symlink `T-whisper → t-whisper`，case-sensitive 磁碟也能用大寫。
- README / `--help` / repo 用大寫 `T-Whisper` 呈現品牌。

## 架構

```
T-Whisper/
├── README.md            安裝 + 用法（繁體）
├── install.sh           curl | bash 入口
├── uninstall.sh
├── VERSION              版本號
├── LICENSE              MIT
├── Makefile             test / lint / install（開發用）
├── bin/t-whisper        唯一主指令（dispatcher）
├── libexec/
│   ├── common.sh        依賴檢查、brew prefix、config、模型路徑、訊息、麥克風解析
│   ├── live.sh          即時轉錄（VAD / --sliding）
│   ├── rec.sh           錄音 → 事後精準轉錄
│   ├── file.sh          轉現成音檔／影片
│   ├── trad.sh          簡轉繁（OpenCC s2twp）
│   ├── models.sh        模型 list / pull / rm + 互動選單
│   ├── doctor.sh        環境自我檢查
│   └── render.py        sliding 即時單行渲染（原 whisper-render）
├── test/sample.wav      煙霧測試音檔
└── .github/workflows/shellcheck.yml
```

## 指令介面

```
t-whisper live [--sliding]      即時轉錄（預設 VAD，最準；--sliding 邊講邊吐）
t-whisper rec                   錄音 → 結束後精準轉錄（最乾淨：絕對時間戳、零重複）
t-whisper file <音檔|影片>      轉現成檔案
t-whisper trad <檔案|資料夾>    簡轉繁
t-whisper models [list|pull <名稱>|rm <名稱>]
t-whisper doctor                檢查依賴／麥克風權限／模型
t-whisper update                更新到最新版
t-whisper --help / --version
```

旗標（覆蓋 config）：`--model turbo|large-v3`、`--lang zh`、`--beam N`、`--mic <名稱關鍵字>`。

## 設定檔 `~/.config/t-whisper/config`

```
lang=zh
model=turbo
beam=5
mic=default
to_traditional=1
```

優先序：CLI 旗標 > 設定檔 > 內建預設。

## 模型

- registry：`turbo`（ggml-large-v3-turbo.bin, ~1.6G）、`large-v3`（ggml-large-v3.bin, ~3.0G），來源 HuggingFace ggerganov/whisper.cpp。
- 存放 `~/.local/share/t-whisper/models/`；安裝時偵測既有 `~/models/whisper/*.bin` 沿用。
- 首次需要模型時互動選單（turbo / large-v3 / 兩者 / 稍後），下載後校驗大小。
- 模型解析順序：`TW_MODELS` → legacy `~/models/whisper`。

## 麥克風選擇

- `mic` 用**名稱關鍵字**比對（比索引穩）；程式跑 `ffmpeg -list_devices` 換算 avfoundation 索引。
- `mic=default` → 由 `system_profiler` 找系統預設輸入裝置名稱再比對。
- `t-whisper rec --mic` 列出所有輸入裝置供選。

## 依賴與錯誤處理

- `common.sh` 的 `need_cmd` 缺依賴時印「錯誤：… / 修復：brew install …」兩行，不丟 stack trace。
- 缺模型 → 互動詢問下載。麥克風權限未開 → 偵測無音並提示到系統設定開啟。

## 安裝流程（install.sh，可重跑）

1. 偵測 macOS + 架構 → brew prefix
2. 無 Homebrew → 提示官方安裝指令（不擅自裝）
3. `brew install whisper-cpp ffmpeg sdl2 opencc`（已裝跳過）
4. 下載 repo 到 `~/.local/share/t-whisper/`，`bin/t-whisper` 連結進 `~/.local/bin/`（+ 大寫 symlink）
5. 檢查 `~/.local/bin` 在 PATH，否則提示加入 `~/.zshrc`
6. 偵測既有模型沿用；跑模型選單
7. 收尾提示 `t-whisper doctor`

## 版本／更新／解除安裝

- `--version` 讀 `VERSION`；`update` 重新 `git pull`（或重跑 install.sh）並顯示版本差異。
- `uninstall` 移除連結與 `~/.local/share/t-whisper/`，詢問是否一併刪模型與 config；不動 brew 依賴。

## 測試（適度，不過度）

- `t-whisper doctor` = 煙霧測試。
- `test/sample.wav` + `make test`：驗證 file → 簡轉繁 → 輸出整鏈通。
- 所有 `.sh` 過 shellcheck（GitHub Actions CI）。
- 不寫即時／音訊單元測試（難測，YAGNI）。

## 授權

MIT，檔頭標示信誌科技。
```
