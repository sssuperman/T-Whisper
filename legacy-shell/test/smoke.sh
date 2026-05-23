#!/usr/bin/env bash
# 煙霧測試：doctor + 用 macOS say 產生範例語音 → file 轉錄 → 驗證有輸出。
# 不committed 二進位音檔，改在測試時即時產生（更乾淨）。
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TW="$ROOT/bin/t-whisper"
fail=0

echo "===== locale 安全（CJK 鄰接變數須加大括號）====="
# zh_TW.UTF-8 等 locale 下，未加大括號的 $var 緊接全形字會把後續位元組吃進變數名 → set -u 報錯。
# 此處掃描整個原始碼，發現未加大括號且緊接非 ASCII 的變數參考就失敗。
if risky=$(grep -rnP '\$([A-Za-z_0-9]+|[?#@*!-])(?=[^\x00-\x7F])' \
      "$ROOT/bin/t-whisper" "$ROOT"/libexec/*.sh "$ROOT/install.sh" "$ROOT/uninstall.sh" 2>/dev/null); then
  echo "❌ 發現未加大括號的變數鄰接非 ASCII（請改 \${var}）："; echo "$risky"; fail=1
else
  echo "✅ 無 locale 風險點"
fi

echo "===== doctor ====="
"$TW" doctor || true   # 環境有缺會非零，煙霧測試不因此中斷

echo "===== trad（簡轉繁）====="
got="$(echo '软体测试项目' | "$TW" trad)"
if [[ "$got" == "軟體測試專案" ]]; then echo "✅ trad: $got"; else echo "❌ trad 預期 軟體測試專案，得到：$got"; fail=1; fi

echo "===== file 轉錄 ====="
if ! command -v say >/dev/null 2>&1; then echo "（無 say，略過）"; exit $fail; fi
sample="$(mktemp -t twsample).aiff"; wav="${sample%.aiff}.wav"
say -o "$sample" "信誌科技語音轉錄測試。" 2>/dev/null || { echo "（say 失敗，略過）"; exit $fail; }
ffmpeg -hide_banner -loglevel error -i "$sample" -ar 16000 -ac 1 -y "$wav" 2>/dev/null

# 先完整擷取再比對：避免 grep -q 提早關閉 pipe 在 pipefail 下造成 SIGPIPE 誤判
mlist="$("$TW" models list 2>&1 || true)"
if printf '%s' "$mlist" | grep -q '✅'; then
  "$TW" file "$wav" >/dev/null 2>&1
  out="${wav%.*}.txt"
  if [[ -s "$out" ]]; then echo "✅ 轉錄輸出存在："; cat "$out"; else echo "❌ 無轉錄輸出"; fail=1; fi
  rm -f "$out"
else
  echo "（尚無模型，略過轉錄測試；先 t-whisper models pull turbo）"
fi
rm -f "$sample" "$wav"
exit $fail
