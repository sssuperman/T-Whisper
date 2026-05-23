#!/usr/bin/env bash
# models.sh — 模型管理：list / pull / rm，以及首次執行的互動選單 picker。

tw_cmd_models() {
  local sub="${1:-list}"; shift 2>/dev/null || true
  case "$sub" in
    list)   tw_models_list ;;
    pull)   tw_models_pull "$@" ;;
    rm)     tw_models_rm "$@" ;;
    picker) tw_models_picker ;;
    *)      die "未知：models $sub" "用 list / pull <turbo|large-v3> / rm <名稱>" ;;
  esac
}

tw_models_list() {
  info "模型狀態（存放：${TW_MODELS}）："
  local name p size
  for name in turbo large-v3; do
    p="$(tw_model_path "$name")"
    size="$(tw_model_size "$name")"
    if [[ -n "$p" ]]; then
      printf '  %s✅ %-9s%s  %s\n' "$C_GRN" "$name" "$C_OFF" "$p" >&2
    else
      printf '  %s⬜ %-9s%s  未下載（約 %s MB）\n' "$C_DIM" "$name" "$C_OFF" "$((size/1024/1024))" >&2
    fi
  done
}

tw_models_pull() {
  local name="${1:-}"
  [[ -n "$name" ]] || die "請指定模型" "t-whisper models pull turbo|large-v3"
  tw_ensure_model "$name" --yes >/dev/null
  tw_models_list
}

tw_models_rm() {
  local name="${1:-}" f
  [[ -n "$name" ]] || die "請指定模型" "t-whisper models rm turbo|large-v3"
  f="$(tw_model_file "$name")"
  [[ -n "$f" ]] || die "未知模型：$name"
  if [[ -f "$TW_MODELS/$f" ]]; then
    rm -f "$TW_MODELS/$f" && ok "已刪除 ${name}（$TW_MODELS/${f}）"
  elif [[ -f "$TW_LEGACY_MODELS/$f" ]]; then
    warn "$name 在舊目錄 ${TW_LEGACY_MODELS}，未自動刪除（避免影響其他用途）"
    info "如確定要刪：rm '$TW_LEGACY_MODELS/$f'"
  else
    warn "$name 本來就沒下載"
  fi
}

# 首次執行的互動選單（install.sh 結尾與首次缺模型時呼叫）
tw_models_picker() {
  # 已有任一模型就略過
  if [[ -n "$(tw_model_path turbo)" || -n "$(tw_model_path large-v3)" ]]; then
    tw_models_list; return 0
  fi
  cat >&2 <<'EOF'

請選擇要下載的模型：
  1) turbo    (約 1.6G) — 快，日常夠用 ★建議
  2) large-v3 (約 3.0G) — 最準，較慢
  3) 兩個都下載
  4) 稍後再說（之後用 t-whisper models pull <名稱>）
EOF
  printf '請輸入 [1]：' >&2
  local ans; read -r ans </dev/tty || ans=""
  case "${ans:-1}" in
    1) tw_ensure_model turbo --yes >/dev/null;    tw_set_config_model turbo ;;
    2) tw_ensure_model large-v3 --yes >/dev/null; tw_set_config_model large-v3 ;;
    3) tw_ensure_model turbo --yes >/dev/null; tw_ensure_model large-v3 --yes >/dev/null; tw_set_config_model large-v3 ;;
    4) info "稍後可用：t-whisper models pull turbo" ;;
    *) warn "未知選項，略過" ;;
  esac
}

# 把選好的模型寫回 config 的 model=
tw_set_config_model() {
  local m="$1"
  [[ -f "$TW_CONFIG" ]] || tw_write_default_config
  if grep -q '^model=' "$TW_CONFIG" 2>/dev/null; then
    sed -i '' "s/^model=.*/model=$m/" "$TW_CONFIG"
  else
    printf 'model=%s\n' "$m" >> "$TW_CONFIG"
  fi
  ok "預設模型設為：$m"
}
