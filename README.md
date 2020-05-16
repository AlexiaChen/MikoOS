# MikoOS

A implementation of OS based-on bochs written in Rust

# Requirement

- Bochs
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

- Build kernel

```bash
cargo rustc -- -Clink-arg=-nostartfiles
```

