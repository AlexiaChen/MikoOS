//! 第 4 章 4-11：有独立栈的第一个内核线程，主动切换、恢复、退出及回收。
//!
//! “正在执行的任务”包含 CPU 寄存器和一条尚未结束的函数调用链。调用链的局部
//! 数据、保存值、返回地址都在栈上；把寄存器与 RSP 换成另一份保存现场，CPU 就能
//! 接着另一条调用链执行。本节只有启动任务和 init 内核线程，没有调度策略/时间片。
//!
//! 两者仍是 ring0，使用同一个 CR3。任务切换与地址空间切换是两个步骤，本节
//! 没有隔离的进程地址空间，也不需要每次重载 CR3、白白冲掉 TLB。
use crate::{arch, console, gate, kprint, memory};
use core::{
    mem::{offset_of, size_of},
    ptr::{addr_of, addr_of_mut, null_mut},
};

const TASK_REGION_SIZE: usize = 32 * 1024;
const DESCRIPTOR_AREA: usize = 4096;
const STACK_CANARY: u64 = 0x4d49_4b4f_5354_414b;

/// C ABI 边界只访问这个字段；六个寄存器和 RIP 已保存在它指向的栈帧中。
#[repr(C)]
struct Context {
    rsp: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ready,
    Running,
    Zombie,
}

#[repr(C)]
struct Task {
    context: Context,
    pid: usize,
    state: State,
    stack_bottom: usize,
    stack_top: usize,
    page_table: u64,
    entry: Option<fn(usize) -> usize>,
    argument: usize,
    progress: usize,
    exit_code: Option<usize>,
}

static mut BOOT_TASK: Task = Task {
    context: Context { rsp: 0 },
    pid: 0,
    state: State::Ready,
    stack_bottom: 0,
    stack_top: 0,
    page_table: 0,
    entry: None,
    argument: 0,
    progress: 0,
    exit_code: None,
};
// 显式记录当前任务，避免原版用 RSP & ~32767 推断任务地址的隐藏前提。
// 启动栈与 IRQ/异常 IST 栈都不等于“32 KiB 对齐的 task_union”，不能直接照搬。
static mut CURRENT: *mut Task = null_mut();
static mut NEXT_PID: usize = 1;
#[cfg(feature = "task-check")]
static mut CHECKED_RETURNS: usize = 0;

/// 物理页凭据一直由启动任务持有；子线程只执行，不拥有可在自身栈上释放的页凭据。
/// 退出与回收分开，才能在另一个仍存活的栈上归还包含任务描述符/栈的整页。
struct KernelThread {
    task: *mut Task,
    backing: memory::PageRun,
}

unsafe extern "C" {
    static boot_stack: u8;
    static boot_stack_top: u8;
    fn task_returned() -> !;
    #[cfg(not(feature = "task-check"))]
    fn task_context_switch(previous: *mut Context, next: *const Context);
    #[cfg(feature = "task-check")]
    fn task_check_switch(previous: *mut Context, next: *const Context) -> u64;
}

const _: () = assert!(offset_of!(Context, rsp) == 0);
const _: () = assert!(size_of::<Context>() == 8);
const _: () = assert!(size_of::<Task>() <= DESCRIPTOR_AREA);

unsafe fn create_kernel_thread(entry: fn(usize) -> usize, argument: usize) -> KernelThread {
    // SAFETY: 单核 IF=0，分配器已初始化；Mapped 的整页仍由初始页表直映。
    unsafe {
        let backing = memory::allocate_pages(memory::AllocationArea::Mapped, 1)
            .expect("no mapped page for kernel thread");
        let base = memory::PAGE_OFFSET + backing.physical_address();
        let task = base as *mut Task;
        let top = base + TASK_REGION_SIZE;
        let pid = NEXT_PID;
        NEXT_PID = NEXT_PID.checked_add(1).expect("PID overflow");
        // 当前分配器的最小单位是 2 MiB。本节沿原版只用其中前 32 KiB，低 4 KiB
        // 放描述符与边界哨兵，其余给栈；其余物理空间仍归整页所有者，不能重复分配。
        // 显式构造，不复制正在运行任务的描述符、链表或整个栈，也就不会携带旧别名。
        task.write(Task {
            context: Context { rsp: top - 64 },
            pid,
            state: State::Ready,
            stack_bottom: base + DESCRIPTOR_AREA + size_of::<u64>(),
            stack_top: top,
            page_table: arch::cr3(),
            entry: Some(entry),
            argument,
            progress: 0,
            exit_code: None,
        });
        ((base + DESCRIPTOR_AREA) as *mut u64).write_volatile(STACK_CANARY);
        // 合成首次切入帧：6 个 GPR + RET 目标 + trampoline 的保护性返回地址。
        // 栈顶 16 字节对齐，消耗 7 个 u64 后 RSP=top-8，满足 C ABI 入口 RSP%16=8。
        // 之后的切换则由真实 CALL 自然留下返回地址，不再合成现场。
        let frame = (top - 64) as *mut [usize; 8];
        frame.write([
            0,
            0,
            0,
            0,
            0,
            0,
            task_trampoline as *const () as usize,
            task_returned as *const () as usize,
        ]);
        kprint!(
            console::GREEN,
            "Kernel thread created: pid={pid}, stack={:#018x}..{top:#018x}\n",
            (*task).stack_bottom
        );
        KernelThread { task, backing }
    }
}

unsafe fn assert_current_stack() {
    let rsp: usize;
    // SAFETY: 只读取当前 RSP，不修改栈或中断状态。
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags))
    }
    // SAFETY: CURRENT 指向仍然存活的任务；这里只读取，不保留跨切换的引用。
    unsafe {
        let task = CURRENT;
        assert!(!task.is_null());
        assert!(((*task).stack_bottom..(*task).stack_top).contains(&rsp));
        assert_eq!(gate::kernel_stack(), (*task).stack_top);
        assert_eq!(arch::cr3(), (*task).page_table);
        assert_eq!((*task).state, State::Running);
    }
}

/// 仅做两任务之间的显式交接。调用期间所有字段通过短期 raw pointer 访问，
/// 不持有 &mut Task 跨越切换；暂停期间另一个任务也会改这些描述符。
unsafe fn switch_to(next: *mut Task) {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 单核 IF=0，previous/next 都存活且有不同的栈；没有借用跨交接边界。
    unsafe {
        let previous = CURRENT;
        assert!(!previous.is_null() && !next.is_null() && previous != next);
        assert_eq!((*next).state, State::Ready);
        assert_eq!((*previous).page_table, (*next).page_table);
        if (*previous).state == State::Running {
            (*previous).state = State::Ready;
        } else {
            assert_eq!((*previous).state, State::Zombie);
        }
        (*next).state = State::Running;
        gate::set_kernel_stack((*next).stack_top);
        CURRENT = next;
        let previous_context = addr_of_mut!((*previous).context);
        let next_context = addr_of!((*next).context);
        #[cfg(not(feature = "task-check"))]
        task_context_switch(previous_context, next_context);
        #[cfg(feature = "task-check")]
        {
            assert_eq!(
                task_check_switch(previous_context, next_context),
                0,
                "task context corrupted"
            );
            CHECKED_RETURNS += 1;
        }
        // 这里在“本任务后来再次被切回来”时才执行，不是新任务第一次开始的位置。
        let current = CURRENT;
        assert_eq!(current, previous);
        assert_current_stack();
    }
}

extern "C" fn task_trampoline() -> ! {
    // SAFETY: 首次帧只能由 create_kernel_thread 构造；已经更新 CURRENT/TSS 并换到新栈。
    unsafe {
        assert!(!arch::interrupts_enabled());
        assert_current_stack();
        let task = CURRENT;
        let entry = (*task).entry.expect("kernel thread has no entry");
        let argument = (*task).argument;
        let code = entry(argument);
        // 用户提供的线程函数正常返回后，其局部资源已结束；再记录 Zombie 和退出码。
        // 不能在这个栈上 free_pages：当前 RET/局部变量也住在同一页里。
        (*task).exit_code = Some(code);
        (*task).state = State::Zombie;
        kprint!(
            console::GREEN,
            "Kernel thread exit: pid={} code={code}\n",
            (*task).pid
        );
        switch_to(addr_of_mut!(BOOT_TASK));
    }
    panic!("zombie task resumed");
}

#[unsafe(export_name = "task_unexpected_return")]
extern "C" fn unexpected_return() -> ! {
    panic!("kernel thread trampoline returned");
}

fn init(argument: usize) -> usize {
    // SAFETY: trampoline 只在存活的 init 线程上调用本函数，始终 IF=0。
    unsafe {
        assert_current_stack();
        assert_eq!((*CURRENT).pid, 1);
        assert_eq!(argument, 10);
        kprint!(
            console::YELLOW,
            "init task is running, arg={argument:#018x}\n"
        );
        let local = core::hint::black_box(0x1020_3040_5060_7080u64);
        (*CURRENT).progress = 1;
        #[cfg(feature = "task-check")]
        {
            // 在独立任务栈上走一次 IRQ 门/IST3/IRETQ，再确认回到原任务栈。
            // PIC 此时全屏蔽，软件 IRQ7 只走 ISR 为空的伪中断分支，不冒充设备事件。
            core::arch::asm!("int 0x27");
            assert_current_stack();
        }
        switch_to(addr_of_mut!(BOOT_TASK));
        assert_eq!(core::hint::black_box(local), 0x1020_3040_5060_7080);
        (*CURRENT).progress = 2;
        kprint!(console::YELLOW, "init task resumed after yielding\n");
    }
    1
}

/// # Safety
/// 单核 IF=0，GDT/TSS/页分配器已初始化，PIC 全屏蔽且 IRQ0 演示结束；仅启动调用一次。
pub unsafe fn demonstrate() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 描述符/上下文仅由两个合作的 ring0 执行流访问，全部交接保持 IF=0。
    unsafe {
        assert!(CURRENT.is_null());
        let boot = addr_of_mut!(BOOT_TASK);
        (*boot).stack_bottom = addr_of!(boot_stack) as usize;
        (*boot).stack_top = addr_of!(boot_stack_top) as usize;
        (*boot).page_table = arch::cr3();
        (*boot).state = State::Running;
        CURRENT = boot;
        gate::set_kernel_stack((*boot).stack_top);
        assert_current_stack();
        let child = create_kernel_thread(init, 10);
        switch_to(child.task);
        assert_eq!((*child.task).state, State::Ready);
        assert_eq!((*child.task).progress, 1);
        assert_eq!((*child.task).exit_code, None);
        kprint!(console::GREEN, "Bootstrap task resumed: init yielded\n");
        switch_to(child.task);
        assert_eq!((*child.task).state, State::Zombie);
        assert_eq!((*child.task).progress, 2);
        assert_eq!((*child.task).exit_code, Some(1));
        let guard = ((*child.task).stack_bottom - size_of::<u64>()) as *const u64;
        assert_eq!(
            guard.read_volatile(),
            STACK_CANARY,
            "kernel thread stack overflow"
        );
        let current = CURRENT;
        assert_eq!(current, boot);
        // 确认 CPU 与 TSS 都已回到启动任务，之后才能销毁子线程描述符并归还整页。
        assert_current_stack();
        let physical = child.backing.physical_address();
        memory::free_pages(child.backing);
        // 再申请一页必须回到刚释放的位置，证明线程页已从占用状态恢复空闲。
        let recycled = memory::allocate_pages(memory::AllocationArea::Mapped, 1).unwrap();
        assert_eq!(recycled.physical_address(), physical);
        memory::free_pages(recycled);
        #[cfg(feature = "task-check")]
        {
            let checked = CHECKED_RETURNS;
            assert_eq!(checked, 3);
            kprint!(
                console::GREEN,
                "Task context checks OK: callee-saved GPRs, RSP, stack canaries; child IST return\n"
            );
        }
        kprint!(
            console::GREEN,
            "Kernel thread demo OK: yield, resume, exit=1; page reclaimed\n"
        );
    }
}
