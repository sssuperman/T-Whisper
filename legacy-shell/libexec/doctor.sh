#!/usr/bin/env bash
# doctor.sh — 環境自我檢查（依賴 / 模型 / 麥克風 / PATH）。安裝後先跑這個。

tw_cmd_doctor() {
  local problems=0

  info "T-Whisper 環境檢查"
  printf -- '------------------------------------------------------------\n' >&2

  # Homebrew
  local prefix; prefix="$(tw_brew_prefix)"
  if [[ -n "$prefix" ]]; then ok "Homebrew（${prefix}）"; else warn "找不到 Homebrew"; problems=$((problems+1)); fi

  # 依賴
  local c pkg
  for c in "whisper-cli:whisper-cpp" "whisper-stream:whisper-cpp" "ffmpeg:ffmpeg" "opencc:opencc" "python3:python"; do
    cmd="${c%%:*}"; pkg="${c##*:}"
    if command -v "$cmd" >/dev/null 2>&1; then
      ok "$cmd"
    else
      warn "缺 $cmd → brew install $pkg"; problems=$((problems+1))
    fi
  done

  # 模型
  local has_model=0 name p
  for name in turbo large-v3; do
    p="$(tw_model_path "$name")"
    if [[ -n "$p" ]]; then ok "模型 ${name}（${p}）"; has_model=1; fi
  done
  if [[ $has_model == 0 ]]; then
    warn "尚無模型 → t-whisper models pull turbo"; problems=$((problems+1))
  fi

  # 麥克風（能列出裝置即視為可存取；權限未開通常列不到或抓不到音）
  if command -v ffmpeg >/dev/null 2>&1; then
    local nmic; nmic="$(tw_list_mics | grep -c .)"
    if [[ "${nmic:-0}" -gt 0 ]]; then
      ok "麥克風可列出（$nmic 個輸入裝置）"
    else
      warn "列不到麥克風 → 系統設定 → 隱私權與安全性 → 麥克風，開啟終端機權限"; problems=$((problems+1))
    fi
  fi

  # PATH
  case ":$PATH:" in
    *":$HOME/.local/bin:"*) ok "PATH 已含 ~/.local/bin" ;;
    *) warn "PATH 未含 ~/.local/bin。請在 ~/.zshrc 末端加：export PATH=\"\$HOME/.local/bin:\$PATH\""; problems=$((problems+1)) ;;
  esac

  # 設定檔
  if [[ -f "$TW_CONFIG" ]]; then ok "設定檔（${TW_CONFIG}）"; else warn "尚無設定檔（首次執行會自動建立）"; fi

  printf -- '------------------------------------------------------------\n' >&2
  if [[ $problems == 0 ]]; then
    ok "一切就緒！試試：t-whisper rec"
  else
    warn "有 $problems 項待處理（見上方修復指引）"
    return 1
  fi
}
