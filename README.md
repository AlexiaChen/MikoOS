# MikoOS

A implementation of OS based-on bochs written C

# Requirement

- Bochs 2.6.11
- Bochs-x
- NASM
- bximage

```bash
# ubuntu is better, it also works on WSL2 for windows
sudo apt install nasm bochs bochs-x bximage clang-format
```

# Build Kernel and Floppy Image

- Build and Burn kernel into floppy

```bash
make
```

# Run Kernel floppy image

- Boot

```bash
# run kernel on bochs for linux
make run
```

```shell
# in the interative bochs cli
bochs> continue

# exit
bochs> exit
```


