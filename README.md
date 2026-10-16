# MikoOS

一个从零学习操作系统的小项目，跟着《一个64位操作系统的设计与实现》逐步搭建。
内核主要用 Rust 编写；启动、切换 CPU 模式和进入异常处理时，保留必要的汇编。
代码注释尽量讲清背后的原理，方便边读边学。

目前推进到第 6 章处理器体系结构：能启动、显示文字、管理物理内存，接收时钟和键盘中断，
也能运行内核线程，并让一小段 Rust 程序在权限受限的用户态执行、向内核请求输出文字。
启动演示完成后会等待键盘输入，显示按键的扫描码；还没有命令行，也不能加载外部应用。

## 已经做了什么

| 部分 | 现在能做什么 |
| --- | --- |
| 启动 | 从 FAT12 软盘镜像加载内核，让 CPU 进入 64 位模式 |
| 屏幕 | 在图形帧缓冲上显示文字和格式化日志 |
| 异常处理 | 出现除零、访问未映射地址等 CPU 异常时，显示现场信息 |
| 物理内存 | 读取 BIOS 提供的内存地图，保护内核占用的区域，按 2 MiB 分配和释放连续页 |
| 外部中断 | 设置 PIC 中断控制器，接收两次时钟中断后屏蔽时钟，再开放键盘 |
| 键盘输入 | 接收 PS/2 键盘 IRQ1，显示按下和松开的扫描码字节；还不转换成字符 |
| 内核线程 | 用独立栈运行 init 线程，演示让出、恢复、退出，并回收它占用的内存 |
| 用户态 | 执行内置 Rust 小程序，限制它能访问的内存，完成后回到内核 |
| 系统调用 | 用户程序通过 SYSENTER 请求内核输出文字，收到结果后继续执行 |
| 处理器模式 | 启动时核对 CPU 已进入 64 位内核模式，使用预期页表与高半区栈 |

“分配了物理内存”不等于“已经能直接读写”。CPU 还需要页表来找到它；
目前只直接映射了前 10 MiB 的 RAM。更完整的内存映射会在后续逐步补上。

现在的任务切换由代码主动发起，还没有按时间片自动轮换的调度器。
线程演示里的两个任务都在内核里运行；随后另做一次用户态实验。
用户代码可以读写自己的栈，但不能读取内核、改写代码或执行栈里的数据。
系统调用让用户程序请求内核代为完成受限操作。现在只提供有长度上限的文字输出，
会拒绝非法缓冲区和未实现的调用号；完整的进程管理和应用加载还会在后续继续实现。

## 怎么运行

需要 Rust 1.97.1、`x86_64-unknown-none` 目标、NASM、GNU objcopy 和 Python 3。
模拟运行需要 Bochs，现有配置选择 Intel CPU，以对应教材的 SYSEXIT64 用户态入口。
当前构建和启动在 Linux 下验证，生成镜像不需要 sudo 或挂载软盘。

```sh
rustup target add x86_64-unknown-none --toolchain 1.97.1
make check              # 检查格式和编译器提示
make                    # 生成 bin/boot.img
make run                # 启动 Bochs，调试提示下输入 c 继续，q 退出
```

工具安装位置和 Bochs 的 ROM 配置见[构建说明](doc/context/development/build.md)。
普通镜像会持续接收键盘输入。比如按下再松开 `a`，会分别显示 `0x1e` 和 `0x9e`；
它们表示按键变化，不是字母的 ASCII 编码。扩展键可能发出多个字节。

不打开桌面窗口也可以检查启动过程，以及单独体验内存检查和异常演示：

```sh
make smoke                         # 普通启动
make smoke FEATURES=memory-check   # 分配、释放、碎片和耗尽检查
make smoke FEATURES=irq-check      # 中断返回后，寄存器和标志位是否保持原样
make smoke FEATURES=keyboard-check # 自动发送 a/b 按下与松开，检查真实 IRQ1 接收
make smoke FEATURES=task-check     # 任务切换后，保存的寄存器和栈数据是否正确
make smoke FEATURES=syscall-check  # 系统调用往返、寄存器恢复和错误输入检查
make smoke FEATURES=fault-divide   # 主动触发 CPU 除零异常
make smoke FEATURES=fault-page     # 主动访问未映射地址
make smoke FEATURES=fault-user-kernel     # 用户程序试读内核内存
make smoke FEATURES=fault-user-write      # 用户程序试改自己的代码
make smoke FEATURES=fault-user-exec       # 用户程序试执行栈中的内容
make smoke FEATURES=fault-user-privileged # 用户程序试执行特权指令 HLT
make                              # 演示后恢复普通启动镜像
```

四种用户态故障演示会显示 CPU 拒绝操作的原因，然后停机；每次只启用一种。
普通启动不会主动制造异常。当前无桌面检查能确认内核执行和日志输出。
普通 smoke 到达键盘等待入口后就退出，实际输入由 `keyboard-check` 经本机 VNC 发送并验证。

Bochs 的 RFB 后端无法显示这里的 32 位色深画面，所以这些检查不代表屏幕效果已经验证；尚未测试真机。

## 接下来学什么

第 6 章已把 CPU 模式和地址空间机制对应到现有启动代码，并增加只读状态检查。
学习说明见[处理器体系结构](doc/context/architecture/processor.md)。下一步进入第 7 章，完善 BootLoader。
键盘的字符转换与组合键处理会在后续设备驱动章节继续完善。
后续还需要逐步补齐：

- 任务调度：让多个任务轮流使用 CPU。
- 用户进程和更多系统调用：加载普通程序，管理它的运行，并提供更多内核服务。
- 完整页表和堆：管理更多内存，支持比 2 MiB 页更小的日常分配。
- 设备驱动：完善键盘输入，并支持鼠标和磁盘等设备。
- 文件系统：让内核运行后也能打开、读取和保存文件。现在的 FAT12 读取只服务于启动。

详细章节对应见[开发进度](doc/context/project/progress.md)，
已经实际执行的验证见[验证记录](doc/context/development/validation.md)。
本地对照学习的原版代码位于父目录 `The-design-and-implementation-of-a-64-bit-os`。
