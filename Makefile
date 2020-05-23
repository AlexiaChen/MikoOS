build:
	rm -rf bin
	mkdir bin
	nasm src/boot/boot.asm -o boot.bin
	mv boot.bin bin
	bximage -fd -size=1.44 boot.img
	mv boot.img bin
	dd if=bin/boot.bin of=bin/boot.img bs=512 count=1 conv=notrunc

copyloader:
	nasm src/boot/loader.asm -o loader.bin
	mv loader.bin bin
	sudo mount bin/boot.img /media/ -t  vfat -o loop
	sudo cp bin/loader.bin /media/
	sync
	sudo umount /media/