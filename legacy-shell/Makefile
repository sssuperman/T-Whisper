.PHONY: help lint test install uninstall

help:
	@echo "make lint      — shellcheck 所有腳本"
	@echo "make test      — 煙霧測試（doctor + 範例音檔轉錄）"
	@echo "make install   — 從本機原始碼安裝"
	@echo "make uninstall — 解除安裝"

lint:
	@shellcheck --severity=warning -x bin/t-whisper libexec/*.sh install.sh uninstall.sh test/smoke.sh

test:
	@bash test/smoke.sh

install:
	@bash install.sh

uninstall:
	@bash uninstall.sh
