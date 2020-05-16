# MikoOS

A implementation of OS based-on bochs written in Rust

# Requirement

- Bochs 2.6.11
- NASM
- bximage

```bash
sudo apt install nasm bochs bximage
```

# Build

- Build boot

```bash
nasm boot.asm -o boot.bin
```

- Create Floppy Disk Image:

```
bximage -fd -size=1.44 boot.img
```

- Write boot program into Floppy Disk's MBR sector

```bash
dd if=boot.bin of=boot.img bs=512 count=1 conv=notrunc
```

- Build kernel

```bash
cargo rustc -- -Clink-arg=-nostartfiles
```

