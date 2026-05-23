#!/usr/bin/env bash
# rec.sh — 錄音 + 即時粗略預覽，Ctrl+C 後用主模型整段精準轉錄（最乾淨：絕對時間戳、零重複）。

tw_cmd_rec() {
  local a
  for a in "$@"; do [[ "$a" == "--list-mics" || "$a" == "--mics" ]] && { tw_print_mics; return 0; }; done

  need_cmd ffmpeg ffmpeg
  need_cmd whisper-stream whisper-cpp
  need_cmd whisper-cli whisper-cpp
  [[ "$TW_TRAD" == "1" ]] && need_cmd opencc opencc

  local outdir="$TW_OUTDIR"; mkdir -p "$outdir"
  local stamp wav trad model preview_model micidx threads
  stamp=$(date +%Y%m%d-%H%M%S)
  wav="$outdir/$stamp.wav"; trad="$outdir/$stamp.txt"
  threads=$(tw_threads)
  model="$(tw_resolve_model "$TW_MODEL")"
  preview_model="$(tw_model_path turbo)"; [[ -z "$preview_model" ]] && preview_model="$model"
  micidx="$(tw_resolve_mic_index "$TW_MIC")"

  _tw_done=0; _ffpid=""
  _rec_stop() {
    [[ "$_tw_done" == "1" ]] && return; _tw_done=1
    [[ -n "$_ffpid" ]] && kill -INT "$_ffpid" 2>/dev/null
    [[ -n "$_ffpid" ]] && wait "$_ffpid" 2>/dev/null
    if [[ -s "$wav" ]]; then
      tw_transcribe "$wav" "$trad" "$model"
      printf '原始錄音：%s\n' "$wav" >&2
    else
      warn "沒有錄到音訊（檢查麥克風權限：系統設定 → 隱私權與安全性 → 麥克風）"
    fi
    exit 0
  }
  trap _rec_stop INT TERM

  printf '● 錄音中 → %s\n' "$wav" >&2
  printf '裝置: [%s] %s  |  轉錄模型: %s  |  語言: %s\n' \
    "$micidx" "$(tw_list_mics | awk -F'\t' -v i="$micidx" '$1==i{print $2}')" \
    "$(basename "$model")" "$TW_LANG" >&2
  printf '下方為即時粗略字幕（僅供參考，最終以結束後的精準轉錄為準）\n' >&2
  printf '按 Ctrl+C 停止錄音並開始精準轉錄。\n' >&2
  printf -- '------------------------------------------------------------\n' >&2

  ffmpeg -hide_banner -loglevel error -f avfoundation -i ":$micidx" -ar 16000 -ac 1 -y "$wav" >/dev/null 2>&1 &
  _ffpid=$!

  T_WHISPER_TRAD="$TW_TRAD" \
  whisper-stream -m "$preview_model" -l "$TW_LANG" -t "$threads" \
    --step 700 --length 7000 --keep 200 -ac 0 -kc 2>/dev/null \
    | python3 "$TW_LIBEXEC/render.py" || true

  _rec_stop
}
