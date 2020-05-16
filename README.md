# MikoOS

A implementation of OS based-on bochs written in Rust

# Requirement

- Bochs
- NASM

```bash
sudo apt install nasm
```

# Build

```bash
cargo rustc -- -Clink-arg=-nostartfiles
```

