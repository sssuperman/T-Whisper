#!/usr/bin/env bash
# update.sh — 更新到最新版（git pull repo，或重跑安裝程式）。

tw_cmd_update() {
  local old; old="$(cat "$TW_ROOT/VERSION" 2>/dev/null || echo "?")"
  if [[ -d "$TW_ROOT/.git" ]] && command -v git >/dev/null 2>&1; then
    info "更新中（git pull）…"
    git -C "$TW_ROOT" pull --ff-only || die "git pull 失敗" "手動到 $TW_ROOT 解決衝突後再試"
  else
    die "此安裝非 git 版本，無法自動更新" "重跑安裝指令：curl -fsSL <repo>/install.sh | bash"
  fi
  local new; new="$(cat "$TW_ROOT/VERSION" 2>/dev/null || echo "?")"
  chmod +x "$TW_ROOT/bin/t-whisper" 2>/dev/null
  if [[ "$old" == "$new" ]]; then
    ok "已是最新版（${new}）"
  else
    ok "已更新：$old → $new"
  fi
}
