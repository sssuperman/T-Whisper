#!/usr/bin/env bash
# T-Whisper 解除安裝。移除指令連結與程式；詢問是否一併刪模型與設定。
set -uo pipefail

TW_ROOT="${TW_HOME:-$HOME/.local/share/t-whisper}"
BIN_DIR="$HOME/.local/bin"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/t-whisper"

say()  { printf '\033[36m▶ %s\033[0m\n' "$*"; }
ok()   { printf '\033[32m✅ %s\033[0m\n' "$*"; }

say "移除指令連結…"
rm -f "$BIN_DIR/t-whisper" "$BIN_DIR/T-whisper"; ok "已移除 t-whisper / T-whisper"

# 模型（在 TW_ROOT/models 內）
if [[ -d "$TW_ROOT/models" ]]; then
  printf '要一併刪除已下載的模型嗎？（%s）[y/N] ' "$(du -sh "$TW_ROOT/models" 2>/dev/null | cut -f1)"
  read -r a </dev/tty || a=""
  [[ "$a" =~ ^[Yy] ]] && { rm -rf "$TW_ROOT/models"; ok "已刪除模型"; } || say "保留模型：$TW_ROOT/models"
fi

say "移除程式…"
rm -rf "$TW_ROOT"; ok "已移除 $TW_ROOT"

# 設定
if [[ -d "$CONFIG_DIR" ]]; then
  printf '要刪除設定檔嗎？（%s）[y/N] ' "$CONFIG_DIR"
  read -r a </dev/tty || a=""
  [[ "$a" =~ ^[Yy] ]] && { rm -rf "$CONFIG_DIR"; ok "已刪除設定"; } || say "保留設定：$CONFIG_DIR"
fi

ok "解除安裝完成。"
say "依賴（whisper-cpp / ffmpeg / sdl2 / opencc）未移除，如不需要可自行：brew uninstall whisper-cpp ffmpeg sdl2 opencc"
