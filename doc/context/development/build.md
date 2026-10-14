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
ROM 默认位置针对本机 Debian/Ubuntu 安装，不同系统需要修改 conf。
conf/win-bochsrc 是历史参考配置，没有在 Windows 上验证 Rust 构建。

```sh
make smoke
make smoke FEATURES=memory-check
make smoke FEATURES=fault-divide
make smoke FEATURES=fault-page
make                  # 演示后恢复正常启动镜像
```

smoke 使用 conf/smoke-bochsrc 的 RFB 后端（不要求桌面，仍创建本地 VNC 监听端口），
检查 0xe9 输出中的阶段标记、内存一致性与故障结果；无客户端也能运行。
普通启动还会进行物理页分配、已映射页实际读写和释放演示。
`memory-check` 在此之后追加实现完成后的分配器验证：零/超大请求、65 页跨位图字、
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

模拟器日志和 smoke 输出写 bin/，不提交；成功时按 normal/fault-divide/fault-page 保存独立日志。失败时先查 bin/bochs-smoke.log 与 bin/smoke-output.log。
Rust panic、CPU 异常、BIOS/Loader 停住是不同层次，要按日志区分。
