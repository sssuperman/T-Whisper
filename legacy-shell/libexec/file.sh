#!/usr/bin/env bash
# file.sh — 轉現成音檔／影片（任何 ffmpeg 讀得到的格式）→ 乾淨繁體逐字稿。

tw_cmd_file() {
  local in="${1:-}"
  [[ -n "$in" ]] || die "請指定音檔或影片" "t-whisper file <檔案>"
  [[ -f "$in" ]] || die "找不到檔案：$in"

  need_cmd whisper-cli whisper-cpp
  need_cmd ffmpeg ffmpeg
  [[ "$TW_TRAD" == "1" ]] && need_cmd opencc opencc

  local model out tmpwav
  model="$(tw_resolve_model "$TW_MODEL")"
  out="${in%.*}.txt"

  # 一律轉成 16k mono WAV（whisper-cli 最佳輸入；也讓 mp3/mp4/m4a… 都能轉）。
  # 用 temp 目錄，結束時整個移除。temp 路徑存全域（非 local），
  # 這樣 EXIT trap 在函式返回後於全域觸發時仍可存取 → 正常結束、die、訊號各路徑都會清乾淨。
  TW_FILE_TMPDIR="$(mktemp -d -t twfile)"
  trap 'rm -rf "${TW_FILE_TMPDIR:-}"' EXIT INT TERM
  tmpwav="$TW_FILE_TMPDIR/audio.wav"
  info "前處理音訊（→ 16k mono）…"
  ffmpeg -hide_banner -loglevel error -i "$in" -ar 16000 -ac 1 -y "$tmpwav" \
    || die "音訊轉檔失敗：$in" "確認檔案格式，或先用 ffmpeg 檢查"

  tw_transcribe "$tmpwav" "$out" "$model"
}
