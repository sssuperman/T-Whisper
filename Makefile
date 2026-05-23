BIN := t-whisper
ARM := aarch64-apple-darwin
X86 := x86_64-apple-darwin
UNIVERSAL := target/universal/$(BIN)

.PHONY: help build universal test lint fmt install uninstall clean

help:
	@echo "make build      — 編譯本機架構（debug 用 cargo run）"
	@echo "make universal  — 編譯 arm64+x86_64 並 lipo 成 universal binary"
	@echo "make test       — cargo test + 用 test/sample.wav 煙霧測試"
	@echo "make lint       — cargo fmt --check + clippy"
	@echo "make install    — 安裝 universal binary 到 ~/.local/bin"
	@echo "make uninstall  — 解除安裝"

build:
	cargo build --release

universal:
	cargo build --release --target $(ARM)
	cargo build --release --target $(X86)
	@mkdir -p target/universal
	lipo -create -output $(UNIVERSAL) \
		target/$(ARM)/release/$(BIN) \
		target/$(X86)/release/$(BIN)
	@echo "✅ universal binary: $(UNIVERSAL)"
	@lipo -info $(UNIVERSAL)

test:
	cargo test
	@echo "=== 煙霧測試：file 轉錄 ==="
	@cargo build --release
	@MODEL=$$(ls $$HOME/models/whisper/ggml-large-v3-turbo.bin 2>/dev/null); \
	if [ -n "$$MODEL" ]; then \
		./target/release/$(BIN) file test/sample.wav --model "$$MODEL" 2>/dev/null && echo "✅ 轉錄鏈通"; \
		rm -f test/sample.txt; \
	else echo "（無模型，略過轉錄煙霧測試）"; fi

lint:
	cargo fmt --check
	cargo clippy --release -- -D warnings

install: universal
	@mkdir -p $$HOME/.local/bin
	install -m 0755 $(UNIVERSAL) $$HOME/.local/bin/$(BIN)
	@echo "✅ 已安裝到 ~/.local/bin/$(BIN)"

uninstall:
	rm -f $$HOME/.local/bin/$(BIN)
	@echo "已移除 ~/.local/bin/$(BIN)"

clean:
	cargo clean
