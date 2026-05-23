#!/usr/bin/env bash
# T-Whisper 解除安裝。
set -uo pipefail
BIN_DIR="$HOME/.local/bin"
DATA="${TW_HOME:-$HOME/.local/share/t-whisper}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/t-whisper"

ok() { printf '\033[32m✅ %s\033[0m\n' "$*"; }
say() { printf '\033[36m▶ %s\033[0m\n' "$*"; }

rm -f "$BIN_DIR/t-whisper"; ok "已移除 ~/.local/bin/t-whisper"

if [[ -d "$DATA/models" ]]; then
  printf '要刪除下載的模型嗎？（%s）[y/N] ' "$(du -sh "$DATA/models" 2>/dev/null | cut -f1)"
  read -r a </dev/tty || a=""
  [[ "$a" =~ ^[Yy] ]] && { rm -rf "$DATA"; ok "已刪除模型"; } || say "保留模型：$DATA/models"
fi

if [[ -d "$CONFIG_DIR" ]]; then
  printf '要刪除設定檔嗎？[y/N] '
  read -r a </dev/tty || a=""
  [[ "$a" =~ ^[Yy] ]] && { rm -rf "$CONFIG_DIR"; ok "已刪除設定"; } || say "保留設定：$CONFIG_DIR"
fi

say "opencc 未移除（其他程式可能也在用）；如需要：brew uninstall opencc"
ok "解除安裝完成"
