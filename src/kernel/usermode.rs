//! 第 5 章 5-1：把“执行普通函数”变成“受 CPU 权限约束的用户程序”。
//!
//! CPL 与地址高低是两回事：SYSEXIT 把 CPL 改成 3，页表 U/S 决定此时能访问
//! 哪些内存。只改变 CS，却把内核页也标成 User，不能得到用户/内核隔离。
//! 原版本节使用宽松映射演示入口；这里保留指令路线，另外建立两个受控的用户页。
use crate::{arch, console, gate, kprint, memory, syscall, trap};
use core::{arch::asm, ptr};

const CODE: u64 = 0x80_0000;
const STACK: u64 = 0x9f_f000;
const STACK_TOP: u64 = 0xa0_0000;
const PAGE: usize = 4096;
const PRESENT: u64 = 1;
const WRITE: u64 = 1 << 1;
const USER: u64 = 1 << 2;
const NX: u64 = 1 << 63;
const BRANCH: u64 = PRESENT | WRITE | USER;
const CANARY: u64 = 0x4d49_4b4f_5553_4552;
const INPUT: u64 = 0x1020_3040_5060_7080;
const RESULT: u64 = INPUT.wrapping_add(17).rotate_left(7);

#[repr(C, align(4096))]
struct Table([u64; 512]);
static mut PDPT: Table = Table([0; 512]);
static mut PD: Table = Table([0; 512]);
static mut PT: Table = Table([0; 512]);
#[repr(C, align(16))]
struct ReturnStack([u8; 32768]);
static mut RETURN_STACK: ReturnStack = ReturnStack([0; 32768]);
static mut ACTIVE: bool = false;

unsafe extern "C" {
    static __user_start: u8;
    static __user_end: u8;
    fn user_enter(entry: u64, stack: u64) -> u64;
    fn user_return_entry();
}

/// 这段 Rust 被单独链接，再复制到 CODE。不能调用内核函数、引用内核静态数据，
/// 也不能产生 panic 路径：链接地址在高半区，而运行地址在低半区，绝对地址不会
/// 自动重定位。volatile 强制生成真实的用户栈读写；常量运算本身无外部依赖。
/// 发布前还要看 .user.text 反汇编，确认没有跨段调用或内核数据访问，不能只看源码。
/// syscall-check 可以把内核地址作为“应拒绝的指针参数”，但不能直接解引用它。
#[unsafe(link_section = ".user.text")]
#[unsafe(no_mangle)]
#[inline(never)]
extern "C" fn user_demo() -> ! {
    let mut values = [INPUT, 0u64];
    // SAFETY: 局部数组在已映射的独占用户栈中，两个元素均已初始化且指针未逃逸。
    let result = unsafe {
        let p = values.as_mut_ptr();
        p.write_volatile(INPUT);
        let input = p.read_volatile();
        p.add(1)
            .write_volatile(input.wrapping_add(17).rotate_left(7));
        p.add(1).read_volatile()
    };
    let _ = result;
    #[cfg(feature = "fault-user-kernel")]
    // SAFETY: 显式故障实验，预期 CPU 拒绝用户读取 Supervisor 内核页；不构造 Rust 引用。
    unsafe {
        asm!("mov rax, [rax]", "ud2", in("rax") memory::PAGE_OFFSET as u64 + 0x10_0000,
            options(noreturn, nostack));
    }
    #[cfg(feature = "fault-user-write")]
    // SAFETY: 显式故障实验，用户代码页只读，预期得到保护性 #PF。
    unsafe {
        asm!("mov byte ptr [rax], 0", "ud2", in("rax") CODE, options(noreturn, nostack));
    }
    #[cfg(feature = "fault-user-exec")]
    // SAFETY: 显式故障实验，栈页 NX，预期在取指阶段得到 #PF，而不是执行栈内容。
    unsafe {
        asm!("jmp rax", in("rax") STACK, options(noreturn, nostack));
    }
    #[cfg(feature = "fault-user-privileged")]
    // SAFETY: 显式故障实验，HLT 仅允许 CPL0；用户执行预期产生 #GP(0)。
    unsafe {
        asm!("hlt", "ud2", options(noreturn, nostack));
    }
    #[cfg(not(any(
        feature = "fault-user-kernel",
        feature = "fault-user-write",
        feature = "fault-user-exec",
        feature = "fault-user-privileged"
    )))]
    {
        // 栈上的定长数组不依赖高半区 .rodata。文字里的格式符也应原样输出。
        let message = *b"Hello World! %s %n\n";
        let mut failures = 0u64;
        failures |= (syscall::request(15, 0, 0) != syscall::ENOSYS) as u64;
        failures |= (syscall::request(128, 0, 0) != syscall::ENOSYS) as u64;
        failures |= (syscall::request(
            syscall::WRITE,
            message.as_ptr() as u64,
            message.len() as u64,
        ) != message.len() as i64) as u64;
        #[cfg(feature = "syscall-check")]
        {
            macro_rules! check {
                ($number:expr, $address:expr, $length:expr, $expected:expr) => {
                    failures |= (syscall::request($number, $address, $length) != $expected) as u64;
                };
            }
            check!(0, 0, 0, syscall::ENOSYS);
            check!(127, 0, 0, syscall::ENOSYS);
            check!(u64::MAX, 0, 0, syscall::ENOSYS);
            check!(syscall::WRITE, 0, 0, 0);
            check!(syscall::WRITE, 0, 1, syscall::EFAULT);
            check!(
                syscall::WRITE,
                memory::PAGE_OFFSET as u64 + 0x10_0000,
                1,
                syscall::EFAULT
            );
            check!(syscall::WRITE, CODE + PAGE as u64, 1, syscall::EFAULT);
            check!(syscall::WRITE, STACK - 1, 2, syscall::EFAULT);
            check!(syscall::WRITE, STACK_TOP - 1, 2, syscall::EFAULT);
            check!(syscall::WRITE, u64::MAX, 2, syscall::EFAULT);
            check!(
                syscall::WRITE,
                message.as_ptr() as u64,
                257,
                syscall::EINVAL
            );
            check!(syscall::WRITE, 0, u64::MAX, syscall::EINVAL);
            // 完整填充一个最大长度缓冲，验证 256 可接受而 257 被拒绝；
            // volatile 保留用户实际写入，也避免生成不可搬迁的外部 memset 调用。
            let mut maximum = core::mem::MaybeUninit::<[u8; syscall::MAX_WRITE]>::uninit();
            let buffer = maximum.as_mut_ptr().cast::<u8>();
            for offset in 0..syscall::MAX_WRITE {
                // SAFETY: 独占的 256 字节栈槽，逐字节初始化；传给内核前全部写完。
                unsafe { buffer.add(offset).write_volatile(b' ') };
            }
            // SAFETY: 同一数组的最后一字节已在上面的循环中初始化。
            unsafe { buffer.add(syscall::MAX_WRITE - 1).write_volatile(b'\n') };
            check!(
                syscall::WRITE,
                buffer as u64,
                syscall::MAX_WRITE as u64,
                syscall::MAX_WRITE as i64
            );
            let bad_utf8 = [0xffu8];
            check!(syscall::WRITE, bad_utf8.as_ptr() as u64, 1, syscall::EINVAL);
            let registers: u64;
            // SAFETY: 相对 CALL 与目标都在同一复制段；汇编保留 C ABI 的非易失
            // 寄存器。它临时设置 DF 测试入口 CLD，并在返回 Rust 前再次清 DF。
            unsafe {
                asm!("call user_syscall_check", lateout("rax") registers, clobber_abi("C"));
            }
            failures |= registers;
        }
        // 在多次 SYSEXIT 之后重新读 CS/SS，完成门的 CPU 现场另作 CPL3 证据。
        // SAFETY: 内核已安装 DPL3 完成门和 TSS.RSP0；该通知只结束有限演示。
        unsafe {
            asm!("xor edi, edi", "xor esi, esi", "mov di, cs", "mov si, ss", "int 0x80", "ud2",
                in("rax") result, in("rdx") failures, options(noreturn));
        }
    }
}

/// # Safety
/// ring0，单核 IF=0，无活动用户演示；内存分配器、IDT/TSS 已初始化。
pub unsafe fn demonstrate() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 调用前置条件保证没有并发页表/演示状态访问；分配凭据持续持有至拆映射后。
    unsafe {
        assert!(!ptr::addr_of!(ACTIVE).read());
        prepare_cpu();
        let run = memory::allocate_pages(memory::AllocationArea::Mapped, 1)
            .expect("no mapped page for user demo");
        let physical = run.physical_address();
        let backing = (memory::PAGE_OFFSET + physical) as *mut u8;
        // 当前分配粒度仍为 2 MiB，只使用其中两个 4 KiB 页。不提前实现小页分配器。
        // 零填充两个暴露给用户的整页，不能泄漏前一位物理页使用者留下的栈或数据。
        ptr::write_bytes(backing, 0, PAGE * 2);
        let start = ptr::addr_of!(__user_start);
        let size = ptr::addr_of!(__user_end).addr() - start.addr();
        assert!((1..=PAGE).contains(&size));
        assert_eq!(user_demo as *const () as usize, start.addr());
        ptr::copy_nonoverlapping(start, backing, size);
        install_mapping(physical);

        let old_stack = gate::kernel_stack();
        let bottom = ptr::addr_of_mut!(RETURN_STACK).cast::<u8>();
        bottom.cast::<u64>().write(CANARY);
        let top = bottom.add(32768).addr();
        // 不能直接用启动栈顶：user_enter 的挂起现场还在启动栈上，CPU 从栈顶
        // 重新压栈会覆盖它。独立返回栈既演示 RSP0，也保住原来的调用链。
        gate::set_kernel_stack(top);
        syscall::enable(top);
        trap::set_user_return_gate(Some(user_return_entry as *const () as u64));
        ptr::addr_of_mut!(ACTIVE).write(true);
        kprint!(
            console::GREEN,
            "User mapping ready: code=0x800000 R-X, stack=0x9ff000 RW-NX; kernel supervisor\n"
        );
        let result = user_enter(CODE, STACK_TOP - 8);
        assert_eq!(result, RESULT);
        assert!(!arch::interrupts_enabled());
        assert_eq!(bottom.cast::<u64>().read(), CANARY);
        let cs: u16;
        asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags));
        assert_eq!(cs, 8);
        ptr::addr_of_mut!(ACTIVE).write(false);
        syscall::disable();
        trap::set_user_return_gate(None);
        gate::set_kernel_stack(old_stack);
        let root = root();
        root.write_volatile(0);
        arch::flush_tlb();
        assert_eq!(root.read_volatile(), 0);
        // 先撤销所有用户虚拟地址并失效 TLB，再释放所有权，否则旧地址会访问复用页。
        memory::free_pages(run);
        kprint!(
            console::GREEN,
            "User demo cleanup OK: ring0 restored, low mapping removed, page reclaimed\n"
        );
    }
}

/// 本节采用 Intel64 的 SYSEXIT64；SEP 位本身不足以保证其他厂商的 long mode
/// 语义相同。明确检查教学目标，避免在不支持的 CPU 上执行后才得到模糊 #UD。
unsafe fn prepare_cpu() {
    // SAFETY: x86-64 支持 CPUID；MSR/CR0 只在 ring0 修改，保留其余控制位。
    unsafe {
        let vendor = core::arch::x86_64::__cpuid(0);
        assert_eq!(
            (vendor.ebx, vendor.edx, vendor.ecx),
            (0x756e6547, 0x49656e69, 0x6c65746e),
            "SYSEXIT64 demo requires GenuineIntel"
        );
        assert_ne!(
            core::arch::x86_64::__cpuid(1).edx & (1 << 11),
            0,
            "missing SEP"
        );
        assert!(core::arch::x86_64::__cpuid(0x8000_0000).eax >= 0x8000_0001);
        assert_ne!(
            core::arch::x86_64::__cpuid(0x8000_0001).edx & (1 << 20),
            0,
            "missing NX"
        );
        let low: u32;
        let high: u32;
        asm!("rdmsr", in("ecx") 0xc000_0080u32, out("eax") low, out("edx") high, options(nostack));
        asm!("wrmsr", in("ecx") 0xc000_0080u32, in("eax") low | (1 << 11), in("edx") high, options(nostack));
        // EFER.NXE 使 PTE bit63 成为禁止取指位；否则这个位是保留位，会变成另一种 #PF。
        // CR0.WP 也约束内核通过只读映射写入。内核直映别名仍为 RW，尚不是全内核 W^X。
        let cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0, options(nostack));
        asm!("mov cr0, {}", in(reg) cr0 | (1 << 16), options(nostack));
    }
}

fn root() -> *mut u64 {
    // CR3 的低位可含控制字段，不是物理地址的一部分；与 memory::init 保持相同掩码。
    (memory::PAGE_OFFSET + (arch::cr3() & 0x000f_ffff_ffff_f000) as usize) as *mut u64
}

unsafe fn install_mapping(physical: usize) {
    // SAFETY: 静态页表 4 KiB 对齐且位于初始直映范围；单核 IF=0，无活动低地址映射。
    unsafe {
        let root = root();
        assert_eq!(root.read_volatile(), 0);
        assert_eq!(root.add(256).read_volatile() & USER, 0);
        let pdpt = ptr::addr_of_mut!(PDPT).cast::<u64>();
        let pd = ptr::addr_of_mut!(PD).cast::<u64>();
        let pt = ptr::addr_of_mut!(PT).cast::<u64>();
        // 各级 U/S 必须全部为 User；任一级为 Supervisor 都会拒绝 CPL3 访问。
        // 中间级 RW 允许栈写入，叶子代码页清 RW。PD[4] 覆盖 8..10 MiB，
        // 改用 4 KiB PTE 才能让代码与栈分别设置权限，二者之间留空避免相互覆盖。
        pdpt.write(((pd.addr() - memory::PAGE_OFFSET) as u64) | BRANCH);
        pd.add(4)
            .write(((pt.addr() - memory::PAGE_OFFSET) as u64) | BRANCH);
        pt.write(physical as u64 | PRESENT | USER);
        pt.add(511)
            .write((physical + PAGE) as u64 | PRESENT | WRITE | USER | NX);
        assert_eq!(pt.add(510).read(), 0); // 栈下方没有映射，形成 guard page。
        assert_eq!(pt.add(1).read(), 0);
        root.write_volatile(((pdpt.addr() - memory::PAGE_OFFSET) as u64) | BRANCH);
        arch::flush_tlb();
    }
}

#[unsafe(no_mangle)]
extern "C" fn user_return_dispatch(frame: &trap::InterruptFrame) -> u64 {
    // SAFETY: 仅专用 0x80 汇编入口调用，CPU 已切到内核栈，IF=0；状态为单核独占。
    unsafe {
        assert!(ptr::addr_of!(ACTIVE).read());
        assert_user_frame(frame);
        assert_eq!(frame.vector, 0x80);
        assert_eq!(frame.error, 0);
        assert!((CODE..CODE + PAGE as u64).contains(&frame.rip));
        let bottom = ptr::addr_of!(RETURN_STACK).addr();
        let top = bottom + 32768;
        let rsp: usize;
        asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags));
        assert!((bottom + 8..top).contains(&rsp));
        assert_eq!(gate::kernel_stack(), top);
        assert_eq!((frame.registers[9], frame.registers[8]), (0x2b, 0x33));
        assert_eq!(frame.registers[14], RESULT);
        assert_eq!(frame.registers[11], 0, "user syscall checks failed");
        kprint!(
            console::GREEN,
            "System call demo OK: SYSENTER/SYSEXIT64 returned to ring3; write=19, ENOSYS=-38\n"
        );
        #[cfg(feature = "syscall-check")]
        kprint!(
            console::GREEN,
            "System call checks OK: GPRs, RSP, flags; number bounds, user ranges, length, UTF-8\n"
        );
        kprint!(
            console::GREEN,
            "User mode OK: CS=0x2b SS=0x33 CPL=3, Rust stack/result verified; INT 0x80 used TSS.RSP0\n"
        );
        frame.registers[14]
    }
}

/// 本节仅有两个固定用户页；验证完整区间属于其中一页，不能把空洞或高半区
/// 当作可读用户内存。此规则与 install_mapping 同属一个模块，映射变更须同步。
pub fn readable(address: u64, length: u64) -> bool {
    let Some(end) = address.checked_add(length) else {
        return false;
    };
    (CODE <= address && end <= CODE + PAGE as u64) || (STACK <= address && end <= STACK_TOP)
}

pub fn valid_syscall(frame: &syscall::Frame) -> bool {
    let bottom = ptr::addr_of!(RETURN_STACK).addr();
    let top = bottom + 32768;
    // SAFETY: 入口已经位于独立内核栈，单核 IF=0；ACTIVE 只由演示前后修改。
    let active = unsafe { ptr::addr_of!(ACTIVE).read() };
    active
        && gate::kernel_stack() == top
        && (bottom + 8..=top - core::mem::size_of::<syscall::Frame>())
            .contains(&(frame as *const _ as usize))
        && (CODE..CODE + user_code_size() as u64).contains(&frame.rdx)
        && (STACK..STACK_TOP).contains(&frame.rcx)
}

fn user_code_size() -> usize {
    ptr::addr_of!(__user_end).addr() - ptr::addr_of!(__user_start).addr()
}

fn assert_user_frame(frame: &trap::InterruptFrame) {
    assert_eq!((frame.cs, frame.ss), (0x2b, 0x33));
    assert_eq!(frame.rflags & ((3 << 12) | (1 << 9) | (1 << 10)), 0);
    assert!((STACK..STACK_TOP).contains(&frame.rsp));
}

#[cfg(any(
    feature = "fault-user-kernel",
    feature = "fault-user-write",
    feature = "fault-user-exec",
    feature = "fault-user-privileged"
))]
pub unsafe fn verify_expected_fault(frame: &trap::InterruptFrame) {
    // SAFETY: 致命异常处理 IF=0，只读活动状态；不会从异常返回或释放仍在使用的页。
    unsafe { assert!(ptr::addr_of!(ACTIVE).read()) };
    assert_user_frame(frame);
    let (vector, error, address, label) = if cfg!(feature = "fault-user-kernel") {
        (
            14,
            5,
            memory::PAGE_OFFSET as u64 + 0x10_0000,
            "kernel read denied",
        )
    } else if cfg!(feature = "fault-user-write") {
        (14, 7, CODE, "code write denied")
    } else if cfg!(feature = "fault-user-exec") {
        (14, 21, STACK, "stack execute denied")
    } else {
        (13, 0, 0, "privileged HLT denied")
    };
    assert_eq!((frame.vector, frame.error), (vector, error));
    if vector == 14 {
        assert_eq!(arch::cr2(), address);
    }
    if cfg!(feature = "fault-user-exec") {
        assert_eq!(frame.rip, STACK);
    } else {
        assert!((CODE..CODE + PAGE as u64).contains(&frame.rip));
    }
    kprint!(console::GREEN, "User protection check OK: {label}; CPL=3\n");
}
