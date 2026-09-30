KERNEL_TARGET=kernel/x86_64-bennu.json
KERNEL_BIN=build/kernel.bin
STAGE1=build/stage1.bin
STAGE2=build/stage2.bin
IMAGE=build/bennu.img

.PHONY: all image clean

all: image

image:
	mkdir -p build
	RUSTFLAGS="-C link-arg=-Tkernel/linker.ld" cargo build --manifest-path kernel/Cargo.toml --target $(KERNEL_TARGET) --release -Zbuild-std=core
	llvm-objcopy -O binary target/$(KERNEL_TARGET)/release/bennu-kernel $(KERNEL_BIN)
	nasm -f bin boot/stage1.asm -o $(STAGE1)
	nasm -f bin boot/stage2.asm -o $(STAGE2)
	truncate -s 2048 $(STAGE2)
	cat $(STAGE1) $(STAGE2) $(KERNEL_BIN) > $(IMAGE)

clean:
	rm -rf build target
