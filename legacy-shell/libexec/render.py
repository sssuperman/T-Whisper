#!/usr/bin/env python3
# render.py — 把 whisper-stream 滑動模式的即時輸出渲染乾淨。
# 問題：whisper-stream 用 \033[2K\r 原地刷新，部分結果一長就超過終端機寬度而換行，畫面一直往下跳。
# 解法：模擬終端機行緩衝，部分結果永遠壓在「單一行」（截到螢幕寬，不換行），講完一句才固定成歷史。
# 環境變數 T_WHISPER_TRAD=0 可關閉簡轉繁。
import sys, os, codecs, subprocess, shutil

_TRAD_ON = os.environ.get('T_WHISPER_TRAD', '1') == '1'
_HAS_OPENCC = shutil.which('opencc') is not None

def to_trad(s):
    if not _TRAD_ON or not _HAS_OPENCC:
        return s
    try:
        r = subprocess.run(['opencc', '-c', 's2twp.json'], input=s,
                            capture_output=True, text=True, timeout=5)
        return r.stdout.rstrip('\n') or s
    except Exception:
        return s

def term_width():
    try:
        return os.get_terminal_size(sys.stderr.fileno()).columns
    except Exception:
        return 100

ESC = '\x1b'
out = sys.stdout
dec = codecs.getincrementaldecoder('utf-8')('ignore')

line = []

def disp_width(s):
    w = 0
    for ch in s:
        o = ord(ch)
        w += 2 if (0x1100 <= o <= 0x115F or 0x2E80 <= o <= 0xA4CF or
                   0xAC00 <= o <= 0xD7A3 or 0xF900 <= o <= 0xFAFF or
                   0xFE30 <= o <= 0xFE4F or 0xFF00 <= o <= 0xFF60 or
                   0xFFE0 <= o <= 0xFFE6) else 1
    return w

def truncate(s, maxw):
    if disp_width(s) <= maxw:
        return s
    res, w = [], 0
    for ch in reversed(s):
        cw = disp_width(ch)
        if w + cw > maxw:
            break
        res.append(ch); w += cw
    return ''.join(reversed(res))

def render_partial():
    w = term_width()
    s = truncate(''.join(line), max(w - 1, 10))
    out.write('\r\x1b[2K' + s)
    out.flush()

def commit():
    global line
    s = ''.join(line).strip()
    out.write('\r\x1b[2K')
    if s:
        out.write(to_trad(s) + '\n')
    out.flush()
    line = []

raw = os.fdopen(sys.stdin.fileno(), 'rb', buffering=0)
try:
    while True:
        chunk = raw.read(256)
        if not chunk:
            break
        text = dec.decode(chunk)
        j = 0
        while j < len(text):
            ch = text[j]
            if ch == ESC:
                k = j + 1
                if k < len(text) and text[k] == '[':
                    k += 1
                    while k < len(text) and not text[k].isalpha():
                        k += 1
                    if k < len(text):
                        code = text[j:k + 1]
                        if code.endswith('K'):
                            line = []
                        j = k + 1
                        continue
                j += 1
                continue
            elif ch == '\r':
                line = []
                j += 1
            elif ch == '\n':
                commit()
                j += 1
            else:
                line.append(ch)
                j += 1
        render_partial()
    commit()
except (KeyboardInterrupt, BrokenPipeError):
    try:
        commit()
    except Exception:
        pass
except Exception:
    pass
