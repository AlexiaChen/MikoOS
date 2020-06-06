# MikoOS

A implementation of OS based-on bochs written C

# Requirement

- Bochs 2.6.11
- NASM
- bximage

```bash
sudo apt install nasm bochs bximage
```

# Build Kernel and Floppy Image

- Build and Burn kernel into floppy

```bash
make
```

# Run Kernel floppy image

- Boot

```bash
# C kernel
bochs -f ./linux-bochsrc
```

