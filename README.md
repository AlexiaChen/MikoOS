# MikoOS

《一个64位操作系统的设计与实现》的逐章学习项目。内核使用 Rust `no_std`，
BIOS/FAT12 引导与必要的机器入口保留少量汇编。当前实现到第 4 章初级物理内存管理部分。
原版学习资料在父目录 `The-design-and-implementation-of-a-64-bit-os`。

AI 开发先读 [AGENTS.md](AGENTS.md)，再按 [上下文树](doc/context/README.md) 导航。
要求不用 TDD、以源码为依据、完成实现后同步对应上下文，并充分解释教学性代码。

需要 Rust 1.97.1、x86_64-unknown-none 目标、NASM、GNU objcopy、Python 3；运行需要 Bochs。
工具和 ROM 配置详见 [构建说明](doc/context/development/build.md)。

```sh
make check              # 格式与严格 Clippy
make                    # 构建 Rust 内核和 FAT12 软盘，无需 sudo
make run                # 交互 Bochs，输入 c 继续
make smoke              # 无桌面 Bochs 启动检查
make smoke FEATURES=fault-divide
make smoke FEATURES=fault-page
make                    # 恢复正常镜像
```

当前能力、未完成部分见 [章节进度](doc/context/project/progress.md)，
真实执行过的检查见 [验证记录](doc/context/development/validation.md)。
