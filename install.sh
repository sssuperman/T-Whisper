#!/usr/bin/env bash
# T-Whisper 安裝程式（macOS，Apple Silicon 與 Intel 通用）
# 一行安裝： curl -fsSL https://raw.githubusercontent.com/<你的GH>/T-Whisper/main/install.sh | bash
#
# whisper.cpp 已編入單一執行檔，故不需安裝 whisper-cpp；只需 opencc（繁體輸出）。
set -euo pipefail

# >>> 發佈前改成你的 repo（或用環境變數 TW_REPO 覆蓋）<<<
REPO="${TW_REPO:-T-Intelligence-tw/T-Whisper}"
BIN_DIR="$HOME/.local/bin"
ASSET_URL="https://github.com/$REPO/releases/latest/download/t-whisper"

say()  { printf '\033[36m▶ %s\033[0m\n' "$*"; }
ok()   { printf '\033[32m✅ %s\033[0m\n' "$*"; }
warn() { printf '\033[33m⚠️  %s\033[0m\n' "$*"; }
err()  { printf '\033[31m錯誤：%s\033[0m\n' "$*" >&2; exit 1; }

[[ "$(uname)" == "Darwin" ]] || err "目前只支援 macOS"
say "平台：macOS $(uname -m)"

# opencc（繁體輸出所需）
if command -v opencc >/dev/null 2>&1; then
  ok "opencc 已安裝"
elif command -v brew >/dev/null 2>&1; then
  say "安裝 opencc（繁體輸出用）…"; brew install opencc
else
  warn "未安裝 Homebrew，略過 opencc。繁體輸出需要它：https://brew.sh 後 brew install opencc"
fi

# 下載 universal binary（curl 下載無 quarantine，Gatekeeper 不會擋）
mkdir -p "$BIN_DIR"
say "下載 t-whisper → $BIN_DIR/t-whisper"
if ! curl -fSL -o "$BIN_DIR/t-whisper" "$ASSET_URL"; then
  err "下載失敗：$ASSET_URL
   （請確認該 repo 已發佈含 t-whisper universal binary 的 release）"
fi
chmod +x "$BIN_DIR/t-whisper"
ok "已安裝 t-whisper（$("$BIN_DIR/t-whisper" --version 2>/dev/null || echo '?')）"

# PATH 檢查
case ":$PATH:" in
  *":$BIN_DIR:"*) ok "$BIN_DIR 已在 PATH" ;;
  *) warn "$BIN_DIR 不在 PATH。請在 ~/.zshrc 末端加一行後重開終端機："
     printf '   export PATH="$HOME/.local/bin:$PATH"\n' ;;
esac

# 首次模型選單
say "選擇要下載的模型…"
"$BIN_DIR/t-whisper" models picker || warn "模型稍後可用 t-whisper models pull turbo 下載"

printf '\n'
ok "T-Whisper 安裝完成"
say "下一步：t-whisper doctor   檢查環境"
say "開始用：t-whisper rec      （錄音→精準逐字稿）／ t-whisper file 影片.mp4"
