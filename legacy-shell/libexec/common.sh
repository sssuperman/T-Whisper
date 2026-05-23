#!/usr/bin/env bash
# common.sh — T-Whisper 共用函式：依賴檢查、brew prefix、設定檔、模型、麥克風、訊息。
# 由 bin/t-whisper 與各 libexec/*.sh source。本檔不單獨執行。

# ---- 路徑 ----
TW_HOME="${TW_HOME:-$HOME/.local/share/t-whisper}"
TW_MODELS="${TW_MODELS:-$TW_HOME/models}"
TW_LEGACY_MODELS="$HOME/models/whisper"          # 舊版手動放的模型，沿用
TW_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/t-whisper"
TW_CONFIG="$TW_CONFIG_DIR/config"
TW_OUTDIR_DEFAULT="$HOME/whisper-transcripts"

# ---- 顏色（非 TTY 時關閉）----
if [[ -t 2 ]]; then
  C_RED=$'\033[31m'; C_YEL=$'\033[33m'; C_GRN=$'\033[32m'; C_DIM=$'\033[2m'; C_OFF=$'\033[0m'
else
  C_RED=''; C_YEL=''; C_GRN=''; C_DIM=''; C_OFF=''
fi

info()  { printf '%s\n' "$*" >&2; }
ok()    { printf '%s✅ %s%s\n' "$C_GRN" "$*" "$C_OFF" >&2; }
warn()  { printf '%s⚠️  %s%s\n' "$C_YEL" "$*" "$C_OFF" >&2; }
# die "錯誤訊息" "修復指引"
die() {
  printf '%s錯誤：%s%s\n' "$C_RED" "$1" "$C_OFF" >&2
  [[ -n "${2:-}" ]] && printf '%s修復：%s%s\n' "$C_DIM" "$2" "$C_OFF" >&2
  exit 1
}

# ---- Homebrew prefix（Apple Silicon /opt/homebrew、Intel /usr/local）----
tw_brew_prefix() {
  if command -v brew >/dev/null 2>&1; then brew --prefix; return; fi
  [[ -x /opt/homebrew/bin/brew ]] && { echo /opt/homebrew; return; }
  [[ -x /usr/local/bin/brew ]] && { echo /usr/local; return; }
  echo ""
}

# ---- 依賴檢查：缺了給可執行的修復指引 ----
need_cmd() {  # need_cmd <指令> <brew套件>
  command -v "$1" >/dev/null 2>&1 && return 0
  die "找不到 ${1}（屬於 ${2}）" "brew install $2"
}

# ---- 執行緒數（取效能核心，最多 8）----
tw_threads() {
  local n
  n=$(sysctl -n hw.perflevel0.logicalcpu 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
  (( n > 8 )) && n=8
  echo "$n"
}

# ---- 設定檔 ----
# 預設值（可被 config、再被 CLI 旗標覆蓋）；由 source 進來的子指令使用
# shellcheck disable=SC2034
TW_LANG="zh"; TW_MODEL="turbo"; TW_BEAM="5"; TW_MIC="default"; TW_TRAD="1"
# shellcheck disable=SC2034
TW_OUTDIR="$TW_OUTDIR_DEFAULT"
tw_load_config() {
  [[ -f "$TW_CONFIG" ]] || return 0
  local key val
  while IFS='=' read -r key val; do
    key="${key// /}"; [[ -z "$key" || "$key" == \#* ]] && continue
    val="${val%%#*}"; val="${val#"${val%%[![:space:]]*}"}"; val="${val%"${val##*[![:space:]]}"}"
    # shellcheck disable=SC2034  # 寫入的變數由子指令使用
    case "$key" in
      lang) TW_LANG="$val" ;;
      model) TW_MODEL="$val" ;;
      beam) TW_BEAM="$val" ;;
      mic) TW_MIC="$val" ;;
      to_traditional) TW_TRAD="$val" ;;
      outdir) TW_OUTDIR="${val/#\~/$HOME}" ;;
    esac
  done < "$TW_CONFIG"
}

tw_write_default_config() {
  mkdir -p "$TW_CONFIG_DIR"
  [[ -f "$TW_CONFIG" ]] && return 0
  cat > "$TW_CONFIG" <<EOF
# T-Whisper 設定（CLI 旗標可覆蓋這裡）
lang=zh            # 轉錄語言（zh / en / ja …）
model=turbo        # 預設模型：turbo（快）或 large-v3（準）
beam=5             # beam search 寬度，越大越準越慢
mic=default        # 麥克風名稱關鍵字（default=系統預設輸入）
to_traditional=1   # 1=輸出簡轉繁（台灣用語），0=不轉
outdir=~/whisper-transcripts
EOF
  ok "已建立設定檔：$TW_CONFIG"
}

# ---- 模型 registry ----
# 名稱 → 檔名|URL|位元組大小
tw_model_file() {
  case "$1" in
    turbo)    echo "ggml-large-v3-turbo.bin" ;;
    large-v3) echo "ggml-large-v3.bin" ;;
    *)        echo "" ;;
  esac
}
tw_model_url() {
  echo "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$(tw_model_file "$1")"
}
tw_model_size() {  # 預期位元組數（校驗用）
  case "$1" in
    turbo)    echo 1624555275 ;;
    large-v3) echo 3095033483 ;;
    *)        echo 0 ;;
  esac
}

# 回傳某模型的本機路徑（找得到才回，順序：TW_MODELS → legacy）；找不到回空字串
tw_model_path() {
  local f; f="$(tw_model_file "$1")"
  [[ -z "$f" ]] && { echo ""; return 1; }
  [[ -f "$TW_MODELS/$f" ]] && { echo "$TW_MODELS/$f"; return 0; }
  [[ -f "$TW_LEGACY_MODELS/$f" ]] && { echo "$TW_LEGACY_MODELS/$f"; return 0; }
  echo ""; return 1
}

# 確保某模型存在；不存在就（互動）下載。回傳路徑到 stdout。
tw_ensure_model() {  # tw_ensure_model <名稱> [--yes]
  local name="$1" yes="${2:-}" p
  p="$(tw_model_path "$name")" && { echo "$p"; return 0; }
  local f size url; f="$(tw_model_file "$name")"; size="$(tw_model_size "$name")"; url="$(tw_model_url "$name")"
  [[ -z "$f" ]] && die "未知模型：$name" "可用：turbo / large-v3"
  if [[ "$yes" != "--yes" ]]; then
    warn "尚未下載模型 ${name}（約 $((size/1024/1024)) MB）"
    printf '要現在下載嗎？[Y/n] ' >&2; read -r ans </dev/tty || ans=""
    [[ "$ans" =~ ^[Nn] ]] && die "需要模型才能轉錄" "t-whisper models pull $name"
  fi
  mkdir -p "$TW_MODELS"
  info "下載 $name → $TW_MODELS/$f"
  curl -L --fail -o "$TW_MODELS/$f" "$url" >&2 || die "下載失敗：$url" "檢查網路後重試 t-whisper models pull $name"
  # 校驗大小（容許 1% 誤差）
  local got; got=$(stat -f %z "$TW_MODELS/$f" 2>/dev/null || echo 0)
  if (( size > 0 )) && (( got < size * 99 / 100 )); then
    rm -f "$TW_MODELS/$f"; die "模型下載不完整（$got / $size bytes）" "重試 t-whisper models pull $name"
  fi
  ok "模型就緒：$name"
  echo "$TW_MODELS/$f"
}

# 解析模型參數（turbo / large-v3 / 完整路徑）→ 本機路徑，必要時下載
tw_resolve_model() {  # tw_resolve_model <model-name-or-path> [--yes]
  local m="$1" yes="${2:-}"
  if [[ -f "$m" ]]; then echo "$m"; return 0; fi          # 直接給路徑
  tw_ensure_model "$m" "$yes"
}

# ---- 麥克風 ----
# 列出 avfoundation 音訊輸入裝置：每行 "索引<TAB>名稱"
# 注意：macOS 內建 awk（BSD）不支援 gawk 的 3-arg match()，故用 2-arg match + substr/gsub。
tw_list_mics() {
  ffmpeg -hide_banner -f avfoundation -list_devices true -i "" 2>&1 \
    | awk '
        /AVFoundation audio devices/ { a=1; next }
        /AVFoundation video devices/ { a=0 }
        a && match($0, /\] \[[0-9]+\] /) {
          idx  = substr($0, RSTART, RLENGTH); gsub(/[^0-9]/, "", idx)
          name = substr($0, RSTART + RLENGTH)
          sub(/[ \t\r]+$/, "", name)
          print idx "\t" name
        }'
}

# 系統預設輸入裝置名稱
tw_default_mic_name() {
  system_profiler SPAudioDataType 2>/dev/null \
    | awk '/^ +[^ ].*:$/{name=$0} /Default Input Device: Yes/{gsub(/^ +| *:$/,"",name); print name; exit}'
}

# 把 mic 設定（名稱關鍵字或 default）解析成 avfoundation 索引
tw_resolve_mic_index() {  # tw_resolve_mic_index <名稱關鍵字|default>
  local want="$1" idx name target
  if [[ "$want" == "default" || -z "$want" ]]; then
    target="$(tw_default_mic_name)"
  else
    target="$want"
  fi
  # 先用 target 做關鍵字比對
  while IFS=$'\t' read -r idx name; do
    [[ -z "$idx" ]] && continue
    if [[ -n "$target" && "$name" == *"$target"* ]]; then echo "$idx"; return 0; fi
  done < <(tw_list_mics)
  # 比對不到 → 取第一個輸入裝置
  tw_list_mics | head -1 | cut -f1
}

# ---- OpenCC 簡轉繁 wrapper（依 to_traditional 設定決定是否轉）----
tw_trad_filter() {  # stdin → stdout
  if [[ "$TW_TRAD" == "1" ]] && command -v opencc >/dev/null 2>&1; then
    opencc -c s2twp.json
  else
    cat
  fi
}

# ---- 整段轉錄（file / rec 共用）：絕對時間戳、無重疊重複 ----
tw_transcribe() {  # tw_transcribe <wav> <out_trad> <model>
  local wav="$1" out="$2" model="$3"
  printf '\n轉錄中（%s + beam %s）…整段重算，請稍候\n' "$(basename "$model")" "$TW_BEAM" >&2
  printf -- '------------------------------------------------------------\n' >&2
  whisper-cli -m "$model" -l "$TW_LANG" -bs "$TW_BEAM" "$wav" 2>/dev/null | tw_trad_filter | tee "$out"
  printf -- '------------------------------------------------------------\n' >&2
  printf '逐字稿：%s\n' "$out" >&2
}

# ---- 列出輸入裝置 ----
tw_print_mics() {
  command -v ffmpeg >/dev/null 2>&1 || die "找不到 ffmpeg" "brew install ffmpeg"
  info "可用麥克風（索引：名稱）："
  tw_list_mics | while IFS=$'\t' read -r i n; do printf '  [%s] %s\n' "$i" "$n" >&2; done
  local d; d="$(tw_default_mic_name)"
  [[ -n "$d" ]] && info "系統預設輸入：$d"
}
