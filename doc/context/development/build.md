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
make smoke FEATURES=syscall-check
make smoke FEATURES=fault-divide
make smoke FEATURES=fault-page
make smoke FEATURES=fault-user-kernel
make smoke FEATURES=fault-user-write
make smoke FEATURES=fault-user-exec
make smoke FEATURES=fault-user-privileged
make                  # 演示后恢复正常启动镜像
```

smoke 使用 conf/smoke-bochsrc 的 RFB 后端（不要求桌面，仍创建本地 VNC 监听端口），
检查 0xe9 输出中的阶段标记、内存一致性与故障结果；无客户端也能运行。
普通启动还会进行物理页分配、已映射页实际读写和释放演示。
第 6 章在异常入口就绪后、内存初始化前只读核对 CR0/CR3/CR4/EFER、CS/LAR 和真实 RIP/RSP，
打印 `Processor mode OK`。全部配置都执行该检查，不需要新 feature；详见 [处理器体系结构](../architecture/processor.md)。
随后初始化 PIC，仅开放 PIT 的 IRQ0 接收两次真实中断，再全屏蔽并关中断运行 init 内核线程。
线程主动让出、恢复、退出并回收整页之后，执行一次有限的 SYSEXIT64 用户态演示，
用户通过 SYSENTER 请求文字输出，检查错误返回，再通过临时完成通知回到内核，
禁用系统调用 MSR、撤映射并回收页，然后初始化 8042/PS/2，仅开放 IRQ1。
普通内核持续等待并显示扫描码。普通/memory-check/irq-check/task-check/syscall-check 的 smoke
在 kernel_keyboard_wait 入口退出；故障演示和有限 keyboard-check 在 kernel_halt 退出。
`irq-check` 追加软件 INT 的通用寄存器/标志/栈恢复及伪 IRQ7/15 分支检查；
真正外部交付仍由 PIT 验证。无时钟服务、调度器或 APIC 支持。
`task-check` 使用两个任务各不相同的 callee-saved 寄存器哨兵，检查三次暂停/恢复的寄存器、RSP 和栈数据；
另在 init 栈上走软件 IRQ7 的 IST 返回。所有配置均检查参数/进度/退出码、TSS/CR3/栈边界和任务页回收。
任务演示没有抢占或通用多任务调度；用户态演示完成后继续普通键盘等待。
所有正常配置都核对 SYSENTER 正常/错误返回、19 字节文字输出、真实 CPL3 完成现场、
用户栈运算、TSS.RSP0 完成通知栈、清理后的 CPL0/低地址撤映射。
`syscall-check` 追加调用号 0/127/u64::MAX、NULL/内核/空洞/跨页/溢出区间、
零/256/257/u64::MAX 长度、坏 UTF-8，以及全部 GPR、RCX/RDX 协议值、RSP/栈哨兵和返回 RFLAGS 检查。
服务约定与尚未验证的返回地址拒绝分支见 [系统调用](../architecture/syscall.md)。
`fault-user-kernel` 读取 Supervisor 内核页，预期 #PF(5)、CR2=0xffff800000100000；
`fault-user-write` 写代码页，预期 #PF(7)、CR2=0x800000；
`fault-user-exec` 跳到 NX 栈，预期 #PF(0x15)、CR2=RIP=0x9ff000；
`fault-user-privileged` 执行 HLT，预期 #GP(0)。四种均检查 CPU 保存的 CS=0x2b/SS=0x33、
用户 RSP 和非特权 RFLAGS，然后停机。故障发生在 PS/2 初始化之前，故不要求键盘 ready。
六个故障 feature 之间编译期互斥；普通镜像不主动触发异常。
用户载荷修改后检查 `objdump -d -j .user.text bin/system`：普通载荷不能有段外调用或内核地址引用；syscall-check 的相对 CALL 只能指向同段检查函数，
该配置还允许把内核地址作为测试参数，但不能直接访问它。
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
两份配置同时指定 `cpu: model=corei7_sandy_bridge_2600k`；原 generic 的 x86_64 配置
默认报告 AuthenticAMD，不适合本节明确采用的 Intel64 SYSEXIT 路线。
CPU 模型以 `bochs --help cpu` 的本机列表为准；不能只修改 vendor 字符串冒充硬件支持。
本机 RFB 不支持 32 bpp 显示，smoke 仅对 RFB 设置 panic=report，并且只接受
`32 bpp graphics mode not supported yet` 这一条已知报告；CPU/设备 panic 仍然失败。
所以 smoke 能验证 CPU 执行、MMIO 写入流程和日志，不能证明 GUI 实际呈现正确。
交互 x 后端需要桌面环境，其可视效果尚未验证。

模拟器日志和 smoke 输出写 bin/，不提交；成功时按 normal 或 feature 名保存独立日志。
失败时先查 bin/bochs-smoke.log 与 bin/smoke-output.log。
Rust panic、CPU 异常、BIOS/Loader 停住是不同层次，要按日志区分。
