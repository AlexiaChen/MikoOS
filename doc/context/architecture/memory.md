# 初级物理内存管理

源码：`src/kernel/memory.rs`；常量 `PAGE_OFFSET=0xffff800000000000`、页大小 2 MiB。
分配器：`src/kernel/memory/allocator.rs`。交接校验由 `bootinfo.rs` 负责，协议见 [E820 交接](bootinfo.md)。
本模块建立物理页元数据、内核预留并提供连续页分配/释放，不扩展启动页表。
5-1 的有限用户映射由 [usermode](usermode.md) 独立建立并清理，不改变本分配器的粒度。

流程：验证 v1 完整交接头/全部 E820 区间与固定布局 → 统计 type=1 RAM → 每段向内对齐到 2 MiB →
建立位图/Page/Zone → 保留低端和内核区域 → 检查计数 → 清除 PML4[0] → 重载 CR3。
E820 条目可无序，地址上界取非零长度条目的最大 end；type=5 或未知类型作为不可用区域保留，
不能当成表结束标志；地址、长度和元数据算术使用检查，失败则 panic 停机。

元数据位于链接器 `_end` 之后：4 KiB 对齐的位图、Page 数组、Zone 数组，末尾 256 字节对齐。
容量必须在最初直映的 10 MiB RAM 内，且整个内核、BSS 与元数据包含在一段 type=1 E820 RAM 中，
所有检查在写入前进行。稀疏物理地址空间的高地址预留区也会占 Page 槽位，
很大的物理地址上界可能导致元数据超限；本阶段明确拒绝此布局，不声称通用大内存支持。

位图一位代表一个 2 MiB 物理页，1=已用或不可分配，0=空闲；初始全 1，
只对完整 RAM 页清零。置位/清位直接写回，用 AND 清位，不用 XOR 翻转。
每个 Page 记录 Zone 指针、物理地址、属性、引用数、创建时间；Zone 保存段范围、
总页数、使用/空闲数和总引用数。Zone 容量按 E820 上限 32 预留，避免原版固定 5 段溢出。
bootinfo 先拒绝任意非空区间重叠，分配器还保留重叠 RAM 页断言，不能重复清位并重复统计。

保留 `[0,end_of_struct)` 覆盖的全部物理页。第 0 页有 BIOS/Loader/内核等混合区域，
即使 E820 无完整 RAM Zone，也单独标记使用，不伪造为从 2 MiB 开始的第一个 Zone，
避免扣减无关 Zone 的空闲计数。`init_page` 保留共享引用的含义；`clean_page` 在最后一个
引用释放时同步清位和计数，并禁止释放内核初始化页和无 Zone 空洞。
`clean_page` 是私有辅助接口，由连续页分配器在整段释放时调用。

运行时核对位图与 Page 属性、使用+空闲=总页数、引用总数等关系，再输出 `Memory invariants OK`。
读取 CR3 时屏蔽表项低 12 位及高属性位，清 PML4[0] 后保留 [256] 高半区条目。
页描述符存在不代表该页已经映射；初始直接可解引用的 RAM 仍只有 10 MiB。
恒等别名原用于低地址启动取指，高半区接管后撤销，既暴露低地址误用，也给用户虚拟空间留位。
这不禁止以后建立低地址用户映射，更不等于单靠撤映射就实现了隔离；权限取决于各级页表 U/S。
原版 4-11/5-1 的清除循环被注释，5-1 还开放了共享映射的用户权限；差异与原因见上述用户态节点。
不保留原 C 含混的 DMA/NORMAL/UNMAPPED 全局索引；后续区域划分章节应按真实映射边界实现。

## 连续物理页分配

对照原版 `code/c04/4-8/kernel/memory.c` 的 `alloc_pages`、`page_init`、`page_clean`，
以及该小节 `main.c` 的连续页演示。当前接口由 memory 模块导出：

- `unsafe allocate_pages(AllocationArea, count) -> Result<PageRun, AllocationError>`。
  `Mapped` 将搜索范围限制在真实直映 RAM `[0,10 MiB)`；`Any` 搜索完整 RAM Zone。
  零页请求返回 `InvalidCount`；无足够连续页返回 `OutOfMemory`，不修改状态。
- `PageRun` 的私有字段记录首个页号与页数，提供物理地址/页数查询，不提供虚拟引用。
  不实现 Copy/Clone，避免重复归还同一凭据；无 Drop，必须显式释放，丢弃会泄漏。
- `unsafe free_pages(PageRun)` 消费凭据，先验证整段归属、占用、单引用、非 KERNEL_INIT，
  再逐页调用 `clean_page` 同步位图/属性/引用/Zone 计数。没有共享页分配和部分释放接口。

算法为每个 Zone 内首次适配的线性逐位扫描；不跨 Zone 拼接，找到整段后才提交。
与原版差异：不用 `p+1` 跨字读取、不移位 64 位、不用 `j+k-1` 计算首个页号，
请求可超过 64 页；使用闭开区间避免区域末尾越界。分配属性固定为 KERNEL|ACTIVE，
只有实际位于初始直映范围的页才附加 PAGE_TABLE_MAPPED；不允许调用者伪造映射属性。

CPU 是否能访问与 RAM 是否已分配是两条独立关系。本接口不建立/解除映射，也不刷新 TLB，
Any 结果不能无条件加 PAGE_OFFSET 解引用；后续分页章节改变映射后应同步此契约。
分配内容未清零。调用前必须已 init，并保证单核、关中断、独占描述符；释放前停止一切页访问。
目前没有锁、SMP/NMI 分配支持，也没有 heap/SLAB 或通用虚拟内存管理。

正常启动分配一个 Mapped 页和一个 Any 页，确认互不重叠，对 Mapped 页首/尾执行真实 volatile
读写后归还两页，打印 `Physical page allocation/read-write/free OK`。该启动演示需要至少两个
可分配物理页，其中至少一个位于直映范围。扩展 `memory-check` 验证见
[构建命令](../development/build.md) 与 [验证记录](../development/validation.md)。

首个内核线程也通过 Mapped 分配一页，保存描述符/独立栈；凭据留在启动任务，
只有返回启动栈并确认线程 Zombie 后才释放，再重新分配同一页确认可复用。
线程仅用整页前 32 KiB，没有新增 SLAB/小块分配器；详见 [任务](tasks.md)。
