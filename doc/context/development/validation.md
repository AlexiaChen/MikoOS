# 验证记录

2026-10-13：迁移基线 `a76a0b8` 的 C 内核到 Rust，未推进未实现的后续章节。
验证环境：Rust 1.97.1、x86_64-unknown-none、Bochs 2.7、legacy BIOS、2 GiB RAM。

## 已执行

- 普通、`fault-divide`、`fault-page` 三种配置的 `make check`：fmt 与 release Clippy（`-D warnings`）全部通过。
- 三种配置的 `make build`：离线构建成功，软盘 1,474,560 字节，Loader 2,301 字节，Kernel 45,288 字节。
  每次镜像均解析 BPB、检查双 FAT 和目录、沿 FAT 链重读两个文件并与输入逐字节比对。
- `python3 scripts/smoke.py`：正常路径完成 E820、内存一致性、移除低地址映射，CPU 到达 `kernel_halt` 断点。
- `python3 scripts/smoke.py --expect fault-divide`：真正触发 CPU #DE，错误码 0，进入 Rust 异常函数并到达停机断点。
- `python3 scripts/smoke.py --expect fault-page`：真正触发 CPU #PF，错误码 0，
  CR2=`0xffff80000aa00000`，只读/内核/未映射页解码正确，进入停机断点。
- 同时启用两个故障 feature：编译按预期拒绝，输出“一次只启用一个异常演示功能”。
- 原 HEAD 的 `font.h` 与当前 `font.rs` 全部 4096 字节一致。
- `nm -u bin/system` 无未解析符号；ELF64 x86-64 入口为 `0xffff800000100000`。
- Python 脚本语法、文档相对链接、`git diff --check` 通过。

最终已执行 `make build` 恢复普通镜像；SHA-256 比对确认 kernel.bin 和 boot.img 与已通过
正常启动验证的版本逐字节相同，不将故障演示镜像留为默认产物。

## 实际日志与解释

```text
OS usable RAM:0x000000007ff9f000; usable 2M pages:1023
Memory invariants OK: free=1023 reserved=1
Identity mapping removed; high-half mapping retained
MikoOS Rust kernel ready
```

可复查日志在 `bin/smoke-normal.log`、`bin/smoke-fault-divide.log`、`bin/smoke-fault-page.log`，
对应模拟器日志在 `bin/bochs-{normal,fault-divide,fault-page}.log`，均为本地生成文件，不提交。
默认机器上第 0 个 2 MiB 页包含内核与元数据，单独保留；完整 RAM Zone 从 2 MiB 开始，
因此 free=1023、reserved=1 并不意味着 Zone 漏记了一页。

## 验证边界

系统自带 `BIOS-bochs-latest` 在 rombios32 中、进入引导扇区之前产生三重异常；
改用系统自带 legacy BIOS 后可启动，交互与 smoke 配置已统一。
本机 RFB 后端报告 `32 bpp graphics mode not supported yet`，仅该组件的该报告被允许；
其余 panic 仍失败。上述结果证明 CPU/页表/异常路径以及 MMIO 写入流程成功，
不代表 GUI 可视呈现已验证。没有验证 x 显示后端、Windows、UEFI、其他模拟器或真机。
上述 2026-10-13 基线中 `clean_page` 尚无上层调用；新增分配器的验证见下节。

## 2026-10-14：连续物理页分配与释放

实现依据为父目录原版 `code/c04/4-8/kernel/memory.c` 和 `main.c`，未修改参考目录。
验证环境沿用 Rust 1.97.1、x86_64-unknown-none、Bochs 2.7、legacy BIOS、2 GiB RAM。

已实际执行：

- 普通、`memory-check`、`fault-divide`、`fault-page` 四种配置的 `make check` 均通过。
- 四种配置的 `make smoke` 均构建成功并通过 FAT12 镜像检查；CPU 均到达 `kernel_halt`。
  普通内核 49,384 字节，内存检查内核 53,552 字节，Loader 仍为 2,301 字节；镜像 1,474,560 字节。
- 普通启动各分配一个 Mapped/Any 页，确认地址不同，对 Mapped 页首尾 volatile 写入并读回，
  然后归还两页，打印 `Physical page allocation/read-write/free OK`。
- `memory-check` 覆盖零页/超大请求、65 页跨 u64 位图边界、不重叠、释放后原位复用、
  映射区耗尽及单页碎片导致的连续分配失败、完整 RAM Zone 耗尽、失败状态不变和最终恢复。
  逐页核对位图、归属、属性、引用数和 Zone 使用/空闲/引用计数，保留页始终为已用。
- #DE 仍为向量 0、错误码 0；#PF 仍为向量 14、错误码 0，CR2=`0xffff80000aa00000`。
- 最后执行普通 `make smoke`，恢复普通镜像并重新确认正常启动，不留故障或扩展检查为默认产物。
- Python 脚本语法、文档相对链接、四份启动日志、`git diff --check` 通过；
  `nm -u bin/system` 无未解析符号，ELF64 x86-64 入口仍为 `0xffff800000100000`。

扩展内存检查的关键输出：

```text
Physical page allocation/read-write/free OK
Page allocator checks OK: cross-word, reuse, fragmentation, exhaustion; free=1023
MikoOS Rust kernel ready
```

记录在 `bin/smoke-memory-check.log`、`bin/bochs-memory-check.log`，正常与故障日志保持独立；均不提交。
首次沙箱内 smoke 因 RFB 无法创建本地套接字而失败，允许模拟器运行后四条路径通过；不是内核故障。

限制：扩展验证明确要求当前单个无预留 RAM Zone、至少 65 个连续空闲页和两个映射区空闲页。
未验证多 Zone/无序 E820/大内存机器的运行分配，不能把本次结果推广到任意 BIOS 布局。
高地址分配只验证记账，未映射 RAM 不作实际读写；普通读写验证只覆盖所分配映射页的首尾。
没有堆、共享页、并发分配、动态页表或性能验证；GUI 与真机限制仍同上。
