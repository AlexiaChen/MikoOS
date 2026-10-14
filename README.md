# MikoOS

一个从零学习操作系统的小项目，跟着《一个64位操作系统的设计与实现》逐步搭建。
内核主要用 Rust 编写；启动、切换 CPU 模式和进入异常处理时，保留必要的汇编。
代码注释尽量讲清背后的原理，方便边读边学。

目前做到第 4 章的初级内存管理：能启动、显示文字、报告 CPU 异常，也能分配和归还物理内存页。
启动演示完成后会停机，还没有命令行，也不能运行普通程序。

## 已经做了什么

| 部分 | 现在能做什么 |
| --- | --- |
| 启动 | 从 FAT12 软盘镜像加载内核，让 CPU 进入 64 位模式 |
| 屏幕 | 在图形帧缓冲上显示文字和格式化日志 |
| 异常处理 | 出现除零、访问未映射地址等 CPU 异常时，显示现场信息 |
| 物理内存 | 读取 BIOS 提供的内存地图，保护内核占用的区域，按 2 MiB 分配和释放连续页 |

“分配了物理内存”不等于“已经能直接读写”。CPU 还需要页表来找到它；
目前只直接映射了前 10 MiB 的 RAM。更完整的内存映射会在后续逐步补上。

## 怎么运行

需要 Rust 1.97.1、`x86_64-unknown-none` 目标、NASM、GNU objcopy 和 Python 3。
模拟运行需要 Bochs。当前构建和启动在 Linux 下验证，生成镜像不需要 sudo 或挂载软盘。

```sh
rustup target add x86_64-unknown-none --toolchain 1.97.1
make check              # 检查格式和编译器提示
make                    # 生成 bin/boot.img
make run                # 启动 Bochs，调试提示下输入 c 继续，q 退出
```

工具安装位置和 Bochs 的 ROM 配置见[构建说明](doc/context/development/build.md)。

不打开桌面窗口也可以检查启动过程，以及单独体验内存检查和异常演示：

```sh
make smoke                         # 普通启动
make smoke FEATURES=memory-check   # 分配、释放、碎片和耗尽检查
make smoke FEATURES=fault-divide   # 主动触发 CPU 除零异常
make smoke FEATURES=fault-page     # 主动访问未映射地址
make                              # 演示后恢复普通启动镜像
```

普通启动不会主动制造异常。当前无桌面检查能确认内核执行和日志输出，
但 Bochs 的 RFB 后端无法显示这里的 32 位色深画面，所以它不代表屏幕效果已经验证；尚未测试真机。

## 接下来学什么

按原版第 4 章的顺序，下一步是让内核接收外部中断，再处理键盘输入，之后学习最初的任务切换。
后续还需要逐步补齐：

- 任务调度：让多个任务轮流使用 CPU。
- 用户态和系统调用：让普通程序与内核分开运行，并能请求内核服务。
- 完整页表和堆：管理更多内存，支持比 2 MiB 页更小的日常分配。
- 设备驱动：读取键盘、鼠标和磁盘等设备。
- 文件系统：让内核运行后也能打开、读取和保存文件。现在的 FAT12 读取只服务于启动。

详细章节对应见[开发进度](doc/context/project/progress.md)，
已经实际执行的验证见[验证记录](doc/context/development/validation.md)。
本地对照学习的原版代码位于父目录 `The-design-and-implementation-of-a-64-bit-os`。
