.DEFAULT_GOAL := build
FEATURES ?=
KERNEL := target/x86_64-unknown-none/release/kernel
CARGO_FEATURES := $(if $(FEATURES),--features $(FEATURES),)

.PHONY: build buildbootloader buildkernel copyimage all run smoke fmt check clean

build: copyimage
	@echo "MikoOS Rust image build completed."

# 保留旧的常用目标名，构建过程无需挂载软盘或 root 权限。
copyimage: buildbootloader buildkernel
	python3 scripts/build_image.py

buildbootloader: bin/boot.bin bin/loader.bin

bin:
	mkdir -p bin

bin/boot.bin: src/boot/boot.asm | bin
	nasm $< -o $@

bin/loader.bin: src/boot/loader.asm src/boot/fat12.inc | bin
	nasm $< -o $@

buildkernel: | bin
	cargo build --release --offline $(CARGO_FEATURES)
	cp $(KERNEL) bin/system
	objcopy -O binary bin/system bin/kernel.bin

all: buildkernel

run: build
	bochs -q -f conf/linux-bochsrc

smoke: build
	python3 scripts/smoke.py $(if $(FEATURES),--expect $(FEATURES),)

fmt:
	cargo fmt

check:
	cargo fmt --check
	cargo clippy --release --offline $(CARGO_FEATURES) -- -D warnings

clean:
	cargo clean
	$(RM) -r bin
