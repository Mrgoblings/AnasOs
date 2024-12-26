.PHONY: run clean

kernel: clean kernel-rust image run

kernel-no-run: clean kernel-rust image

vnc: clean kernel-rust image run-vnc

bootloader: clean bootloader-asm image run

bootloader-no-run: clean bootloader-asm image

test: no-run
	echo "Compiled the OS successfully"

kernel-rust:
	@cd ./anasos-kernel && \
	cargo build --release
	@cp  ./anasos-kernel/target/x86_64-unknown-none/release/anasos-kernel AnasOS/boot/kernel

bootloader-asm:
	@cd ./anasos-kernel/bootloader && \
	nasm -f elf64 header.asm -o header.o && \
	nasm -f elf64 boot-64.asm -o boot-64.o && \
	nasm -f elf64 boot.asm -o boot.o && \
	nasm -f elf64 e820.asm -o e820.o && \
	ld -m elf_x86_64 -T ../linker.ld -o ../../AnasOS/boot/kernel boot.o boot-64.o header.o #e820.o \
	
image:
	grub-mkrescue -o AnasOS.iso AnasOS/

run:
	qemu-system-x86_64 AnasOS.iso

run-vnc:
	qemu-system-x86_64 AnasOS.iso -vnc :0

clean:
	-@cd ./anasos-kernel && cargo clean > /dev/null 2>&1
	-@rm AnasOS/boot/kernel > /dev/null 2>&1
	-@rm AnasOS.iso > /dev/null 2>&1
