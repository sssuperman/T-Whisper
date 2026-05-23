#!/usr/bin/env bash
# live.sh — 即時轉錄。預設 VAD（等停頓、最準）；--sliding 邊講邊吐（turbo、即時）。

tw_cmd_live() {
  local sliding=0 a
  for a in "$@"; do [[ "$a" == "--sliding" ]] && sliding=1; done

  need_cmd whisper-stream whisper-cpp
  [[ "$TW_TRAD" == "1" ]] && need_cmd opencc opencc

  local outdir="$TW_OUTDIR"; mkdir -p "$outdir"
  local stamp simp trad model threads
  stamp=$(date +%Y%m%d-%H%M%S)
  simp="$outdir/$stamp.simp.txt"; trad="$outdir/$stamp.txt"
  threads=$(tw_threads)

  if [[ $sliding == 1 ]]; then
    model="$(tw_resolve_model turbo)"        # 滑動模式求即時 → turbo
  else
    model="$(tw_resolve_model "$TW_MODEL")"
  fi

  _tw_done=0
  # 用 ${simp:-} 等防護：EXIT trap 於函式返回後觸發時 local 已不存在（set -u 會報錯）
  _live_cleanup() {
    [[ "$_tw_done" == "1" ]] && return; _tw_done=1
    local s="${simp:-}" t="${trad:-}"
    if [[ -n "$s" && -s "$s" ]]; then
      tw_trad_filter < "$s" > "$t"
      printf '\n繁體逐字稿已存：%s\n' "$t" >&2
    fi
    [[ -n "$s" ]] && rm -f "$s"
  }
  trap _live_cleanup EXIT INT TERM

  printf '模式: %s  |  模型: %s  |  語言: %s  |  簡→繁: %s\n' \
    "$([[ $sliding == 1 ]] && echo 滑動 || echo VAD)" \
    "$(basename "$model")" "$TW_LANG" "$([[ $TW_TRAD == 1 ]] && echo 開 || echo 關)" >&2
  printf '存檔: %s\n按 Ctrl+C 結束。開始說話…\n' "$trad" >&2
  printf -- '------------------------------------------------------------\n' >&2

  if [[ $sliding == 1 ]]; then
    T_WHISPER_TRAD="$TW_TRAD" \
    whisper-stream -m "$model" -l "$TW_LANG" -t "$threads" \
      --step "${TW_STEP:-700}" --length "${TW_LENGTH:-7000}" --keep 200 -ac "${TW_AC:-0}" -kc \
      -f "$simp" 2>/dev/null | python3 "$TW_LIBEXEC/render.py"
  else
    whisper-stream -m "$model" -l "$TW_LANG" -t "$threads" \
      --step 0 --length 30000 -vth 0.6 -bs "$TW_BEAM" -kc \
      -f "$simp" 2>/dev/null \
      | grep --line-buffered -v -e '^###' -e '^\[Start speaking\]' -e '^[[:space:]]*$' \
      | tw_trad_filter
  fi
}
