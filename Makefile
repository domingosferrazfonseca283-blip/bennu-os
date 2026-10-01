KERNEL_TARGET=kernel/x86_64-bennu.json
KERNEL_BIN=build/kernel.bin
KERNEL_TARGET_NAME=x86_64-bennu
STAGE1=build/stage1.bin
STAGE2=build/stage2.bin
IMAGE=build/bennu.img
MAX_EARLY_KERNEL_SECTORS=120

.PHONY: all image clean

all: image

image:
	mkdir -p build
	RUSTFLAGS="-C link-arg=-T$(CURDIR)/kernel/linker.ld" cargo build --manifest-path kernel/Cargo.toml --target $(KERNEL_TARGET) --release -Zbuild-std=core,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec
	KERNEL_ELF=$$(find target -type f -path '*/release/bennu-kernel' -print -quit); \
	test -n "$$KERNEL_ELF"; \
	llvm-objcopy -O binary "$$KERNEL_ELF" $(KERNEL_BIN)
	@KERNEL_BYTES=$$(wc -c < $(KERNEL_BIN)); \
	KERNEL_SECTORS=$$(( (KERNEL_BYTES + 511) / 512 )); \
	if [ $$KERNEL_SECTORS -gt $(MAX_EARLY_KERNEL_SECTORS) ]; then \
		echo "error: kernel is $$KERNEL_SECTORS sectors; early BIOS loader supports at most $(MAX_EARLY_KERNEL_SECTORS)"; \
		exit 1; \
	fi; \
	nasm -dKERNEL_SECTORS=$$KERNEL_SECTORS -f bin boot/stage2.asm -o $(STAGE2)
	nasm -f bin boot/stage1.asm -o $(STAGE1)
	truncate -s 2048 $(STAGE2)
	cat $(STAGE1) $(STAGE2) $(KERNEL_BIN) > $(IMAGE)

clean:
	rm -rf build target
