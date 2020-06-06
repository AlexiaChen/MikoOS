build: copyimage
	echo "Miko OS Image build completed."

copyimage: buildbootloader buildkernel
	sudo mount -t vfat -o loop bin/boot.img /media/
	sudo cp bin/loader.bin /media/
	sudo cp bin/kernel.bin /media/
	sync
	sudo umount /media/

buildbootloader:
	rm -rf bin
	mkdir bin
	nasm src/boot/boot.asm -o boot.bin
	mv boot.bin bin
	bximage -fd -size=1.44 boot.img
	mv boot.img bin
	dd if=bin/boot.bin of=bin/boot.img bs=512 count=1 conv=notrunc
	nasm src/boot/loader.asm -o loader.bin
	mv loader.bin bin


buildkernel:all

all: bin/system
	objcopy -I elf64-x86-64 -S -R ".eh_frame" -R ".comment" -O binary bin/system bin/kernel.bin

bin/system:	bin/head.o bin/main.o bin/printk.o
	ld -b elf64-x86-64 -z muldefs -o bin/system bin/head.o bin/main.o bin/printk.o -T src/kernel/kernel.lds

bin/main.o:	src/kernel/main.c src/kernel/util.h
	gcc  -static -mcmodel=large -fno-builtin -m64 -c src/kernel/main.c -fno-stack-protector -o bin/main.o

bin/printk.o: src/kernel/printk.c src/kernel/util.h
	gcc -static  -mcmodel=large -fno-builtin -m64 -c src/kernel/printk.c -fno-stack-protector -o bin/printk.o

bin/head.o:	src/kernel/head.S
	gcc -E  src/kernel/head.S > bin/head.s
	as --64 -o bin/head.o bin/head.s

clean:
	rm -rf bin/*