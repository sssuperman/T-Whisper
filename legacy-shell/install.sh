#!/usr/bin/env bash
# T-Whisper 安裝程式（macOS / Apple Silicon + Intel）
# 一行安裝： curl -fsSL https://raw.githubusercontent.com/<你的GH>/T-Whisper/main/install.sh | bash
set -euo pipefail

# >>> 發佈前把這行改成你的公開 repo（也可用環境變數 TW_REPO_URL 覆蓋）<<<
REPO_URL="${TW_REPO_URL:-https://github.com/T-Intelligence-tw/T-Whisper.git}"
TW_ROOT="${TW_HOME:-$HOME/.local/share/t-whisper}"
BIN_DIR="$HOME/.local/bin"

say()  { printf '\033[36m▶ %s\033[0m\n' "$*"; }
ok()   { printf '\033[32m✅ %s\033[0m\n' "$*"; }
warn() { printf '\033[33m⚠️  %s\033[0m\n' "$*"; }
err()  { printf '\033[31m錯誤：%s\033[0m\n' "$*" >&2; exit 1; }

# ---- 1. 平台 ----
[[ "$(uname)" == "Darwin" ]] || err "目前只支援 macOS"
say "平台：macOS $(uname -m)"

# ---- 2. Homebrew ----
if ! command -v brew >/dev/null 2>&1; then
  if [[ -x /opt/homebrew/bin/brew ]]; then eval "$(/opt/homebrew/bin/brew shellenv)"
  elif [[ -x /usr/local/bin/brew ]]; then eval "$(/usr/local/bin/brew shellenv)"
  fi
fi
if ! command -v brew >/dev/null 2>&1; then
  err "找不到 Homebrew。請先安裝：https://brew.sh
   然後重新執行本安裝程式。"
fi
ok "Homebrew（$(brew --prefix)）"

# ---- 3. 依賴 ----
say "安裝依賴（whisper-cpp / ffmpeg / sdl2 / opencc，已裝會跳過）…"
for pkg in whisper-cpp ffmpeg sdl2 opencc; do
  if brew list --versions "$pkg" >/dev/null 2>&1; then ok "$pkg 已安裝"
  else say "brew install $pkg"; brew install "$pkg"; fi
done

# ---- 4. 取得程式碼到 TW_ROOT ----
# 若從本機 clone 執行 install.sh → 直接用該來源；否則 git clone
SELF_DIR=""
if [[ -n "${BASH_SOURCE[0]:-}" && -f "$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)/bin/t-whisper" ]]; then
  SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

if [[ -n "$SELF_DIR" && "$SELF_DIR" != "$TW_ROOT" ]]; then
  say "從本機來源安裝：$SELF_DIR"
  mkdir -p "$TW_ROOT"
  cp -R "$SELF_DIR"/. "$TW_ROOT"/
elif [[ -d "$TW_ROOT/.git" ]]; then
  say "更新既有安裝（git pull）…"; git -C "$TW_ROOT" pull --ff-only || warn "git pull 失敗，沿用現有版本"
elif [[ -z "$SELF_DIR" ]]; then
  command -v git >/dev/null 2>&1 || err "需要 git 來下載。請先：brew install git"
  say "下載程式碼 → $TW_ROOT"
  rm -rf "$TW_ROOT"; git clone --depth 1 "$REPO_URL" "$TW_ROOT" || err "git clone 失敗：$REPO_URL"
fi
chmod +x "$TW_ROOT/bin/t-whisper"

# ---- 5. 連結進 PATH（小寫 canonical + 大寫 alias）----
mkdir -p "$BIN_DIR"
ln -sf "$TW_ROOT/bin/t-whisper" "$BIN_DIR/t-whisper"
ln -sf "t-whisper" "$BIN_DIR/T-whisper"
ok "已安裝指令：t-whisper（大寫 T-whisper 亦可）"

# ---- 6. PATH 檢查 ----
case ":$PATH:" in
  *":$BIN_DIR:"*) ok "$BIN_DIR 已在 PATH" ;;
  *) warn "$BIN_DIR 不在 PATH。請在 ~/.zshrc 末端加一行後重開終端機："
     printf '   export PATH="$HOME/.local/bin:$PATH"\n' ;;
esac

# ---- 7. 設定檔 + 模型選單 ----
"$TW_ROOT/bin/t-whisper" >/dev/null 2>&1 || true   # 觸發 common 初始化
say "建立預設設定檔…"
bash -c "source '$TW_ROOT/libexec/common.sh'; tw_write_default_config" || true
say "選擇要下載的模型…"
"$TW_ROOT/bin/t-whisper" models picker || warn "模型稍後可用 t-whisper models pull turbo 下載"

# ---- 8. 收尾 ----
printf '\n'
ok "T-Whisper 安裝完成（版本 $(cat "$TW_ROOT/VERSION"))"
say "下一步：執行  t-whisper doctor  確認環境"
say "開始用：    t-whisper rec     （錄音→精準逐字稿）"
