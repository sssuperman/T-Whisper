#!/usr/bin/env bash
# trad.sh — 簡轉繁（OpenCC s2twp，含台灣慣用詞）。給匯出的逐字稿做繁體後處理。
# .srt/.vtt 的時間戳行不含中文不受影響。

tw_cmd_trad() {
  need_cmd opencc opencc
  local cfg=s2twp.json

  # stdin 模式：echo "软体" | t-whisper trad
  if [[ ! -t 0 && $# -eq 0 ]]; then opencc -c "$cfg"; return 0; fi
  [[ $# -ge 1 ]] || die "請指定檔案或資料夾" "t-whisper trad <檔案|資料夾> [輸出檔]"

  local target="$1"
  if [[ -d "$target" ]]; then
    shopt -s nullglob
    local f
    for f in "$target"/*.txt "$target"/*.srt "$target"/*.vtt; do
      [[ "$f" == *.trad.* ]] && continue
      opencc -c "$cfg" -i "$f" -o "${f%.*}.trad.${f##*.}" && info "繁體：${f%.*}.trad.${f##*.}"
    done
  elif [[ -f "$target" ]]; then
    local out="${2:-${target%.*}.trad.${target##*.}}"
    opencc -c "$cfg" -i "$target" -o "$out" && info "繁體：$out"
  else
    die "找不到：$target"
  fi
}
