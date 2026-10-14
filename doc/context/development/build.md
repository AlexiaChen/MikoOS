# 构建与运行

从仓库根目录执行。工具：Rust 1.97.1（rust-toolchain.toml）、目标 x86_64-unknown-none、
NASM、GNU objcopy、Python 3；运行需要带 x86_64/VBE 支持的 Bochs 和 BIOS/VGA ROM。
Rust 工具链/目标首次安装需要联网；内核没有第三方 crate，后续构建 `--offline`。

```sh
rustup target add x86_64-unknown-none --toolchain 1.97.1
make check
make
make run
```

`make check` 是 fmt 和 release Clippy（warnings 当错误），不采用 TDD。
`make` 生成 bin/boot.bin、loader.bin、system（带符号 ELF）、kernel.bin、boot.img。
`build_image.py` 用标准库构造 FAT12，核对 BPB、双 FAT、目录、链与文件内容；无 sudo、挂载或 bximage。
Loader 目前需小于 64 KiB；软盘总容量 1.44 MiB。镜像生成不清空已有构建目录。

`make run` 使用 conf/linux-bochsrc 的交互显示配置；Bochs 调试提示下输入 `c` 继续、`q` 退出。
普通镜像初始化后持续显示键盘扫描码；运行中需先返回调试器再输入 q 退出。
ROM 默认位置针对本机 Debian/Ubuntu 安装，不同系统需要修改 conf。
conf/win-bochsrc 是历史参考配置，没有在 Windows 上验证 Rust 构建。

```sh
make smoke
make smoke FEATURES=memory-check
make smoke FEATURES=irq-check
make smoke FEATURES=keyboard-check
make smoke FEATURES=task-check
make smoke FEATURES=fault-divide
make smoke FEATURES=fault-page
make                  # 演示后恢复正常启动镜像
```

smoke 使用 conf/smoke-bochsrc 的 RFB 后端（不要求桌面，仍创建本地 VNC 监听端口），
检查 0xe9 输出中的阶段标记、内存一致性与故障结果；无客户端也能运行。
普通启动还会进行物理页分配、已映射页实际读写和释放演示。
随后初始化 PIC，仅开放 PIT 的 IRQ0 接收两次真实中断，再全屏蔽并关中断运行 init 内核线程。
线程主动让出、恢复、退出并回收整页之后，初始化 8042/PS/2，仅开放 IRQ1。
普通内核持续等待并显示扫描码。普通/memory-check/irq-check/task-check 的 smoke
在 kernel_keyboard_wait 入口退出；故障演示和有限 keyboard-check 在 kernel_halt 退出。
`irq-check` 追加软件 INT 的通用寄存器/标志/栈恢复及伪 IRQ7/15 分支检查；
真正外部交付仍由 PIT 验证。无时钟服务、调度器或 APIC 支持。
`task-check` 使用两个任务各不相同的 callee-saved 寄存器哨兵，检查三次暂停/恢复的寄存器、RSP 和栈数据；
另在 init 栈上走软件 IRQ7 的 IST 返回。所有配置均检查参数/进度/退出码、TSS/CR3/栈边界和任务页回收。
没有抢占、用户态或通用多任务调度；检查通过后继续普通键盘等待。
`keyboard-check` 验证队列 FIFO/环绕/溢出，并由宿主经本次 Bochs 的本机 VNC 发送 a/b 按下与松开，
核对 IRQ1 收到 1e/9e/30/b0 后停机。RFB 握手与按键发送由 Python 标准库完成，无额外依赖；
不请求 framebuffer 更新，不把发送成功当作接收成功，每个事件等待内核日志确认。
`memory-check` 在 PIC 初始化之前追加实现完成后的分配器验证：零/超大请求、65 页跨位图字、
不重叠、释放复用、映射区碎片和耗尽、整段 RAM 耗尽及状态恢复。它不主动触发 CPU 异常。
扩展验证针对当前 2 GiB Bochs 的单个无预留完整 RAM Zone，并需 65 个连续空闲页和至少
两个映射区空闲页；不满足前提会明确 panic，不能用此检查宣称任意 E820 布局均已验证。
每次 smoke 只指定上述单个 feature；扩展检查结束归还全部分配页。
网络沙箱可能禁止本地套接字，需要允许模拟器运行；这不等于内核故障。
本机 Bochs 2.7 配套 latest BIOS 在 rombios32 阶段（进入引导扇区前）异常，
因此交互与 smoke 均使用系统自带 BIOS-bochs-legacy。
本机 RFB 不支持 32 bpp 显示，smoke 仅对 RFB 设置 panic=report，并且只接受
`32 bpp graphics mode not supported yet` 这一条已知报告；CPU/设备 panic 仍然失败。
所以 smoke 能验证 CPU 执行、MMIO 写入流程和日志，不能证明 GUI 实际呈现正确。
交互 x 后端需要桌面环境，其可视效果尚未验证。

模拟器日志和 smoke 输出写 bin/，不提交；成功时按 normal/memory-check/irq-check/keyboard-check/task-check/fault-divide/fault-page 保存独立日志。失败时先查 bin/bochs-smoke.log 与 bin/smoke-output.log。
Rust panic、CPU 异常、BIOS/Loader 停住是不同层次，要按日志区分。
