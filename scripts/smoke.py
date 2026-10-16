#!/usr/bin/env python3
"""验证软盘启动/故障/键盘；在输入等待或最终停机边界退出 Bochs。"""
import argparse
from pathlib import Path
import re
import socket
import struct
import subprocess
import time


def wait_for_marker(process, output, marker, deadline):
    while time.monotonic() < deadline:
        if marker in output.read_text(errors="replace"):
            return
        if process.poll() is not None:
            raise RuntimeError(f"Bochs exited before {marker!r}")
        time.sleep(0.02)
    raise TimeoutError(f"waiting for {marker!r}")


def receive_exact(client, count):
    data = bytearray()
    while len(data) < count:
        chunk = client.recv(count - len(data))
        if not chunk:
            raise RuntimeError("RFB disconnected during handshake")
        data.extend(chunk)
    return bytes(data)


def inject_keyboard(process, output, deadline):
    # 只向本次模拟器发送输入，不在内核注入字节或用软件 INT 冒充真实 IRQ1。
    wait_for_marker(process, output, "PS/2 keyboard ready:", deadline)
    log = Path("bin/bochs-smoke.log").read_text(errors="replace")
    match = re.search(r"listening for connections on port (\d+)", log)
    if not match:
        raise RuntimeError("cannot find this Bochs RFB port")
    client = socket.create_connection(("127.0.0.1", int(match[1])), timeout=3)
    try:
        version = receive_exact(client, 12)
        if not version.startswith(b"RFB 003."):
            raise RuntimeError(f"unsupported RFB version: {version!r}")
        # 使用 RFB 3.3 的无认证握手。无需请求画面更新，键盘输入与 32 bpp 显示无关。
        client.sendall(b"RFB 003.003\n")
        if receive_exact(client, 4) != struct.pack("!I", 1):
            raise RuntimeError("expected local RFB None security")
        client.sendall(b"\x01")  # ClientInit: shared
        header = receive_exact(client, 24)
        name_length = struct.unpack("!I", header[20:24])[0]
        if name_length > 4096:
            raise RuntimeError("invalid RFB server name length")
        receive_exact(client, name_length)
        for key, down, scan in [("a", 1, 0x1e), ("a", 0, 0x9e),
                                ("b", 1, 0x30), ("b", 0, 0xb0)]:
            client.sendall(struct.pack("!BBHI", 4, down, 0, ord(key)))
            # 逐个等实际 IRQ1 消费结果，不以固定延迟或宿主 send 成功判定接收成功。
            wait_for_marker(process, output, f"Keyboard scan byte: {scan:#04x}", deadline)
        return client
    except BaseException:
        client.close()
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    user_faults = {
        "fault-user-kernel": (14, 5, "kernel read denied", "0xffff800000100000"),
        "fault-user-write": (14, 7, "code write denied", "0x0000000000800000"),
        "fault-user-exec": (14, 21, "stack execute denied", "0x00000000009ff000"),
        "fault-user-privileged": (13, 0, "privileged HLT denied", None),
    }
    parser.add_argument("--expect", choices=["fault-divide", "fault-page", "memory-check", "irq-check", "keyboard-check", "task-check", "syscall-check", *user_faults])
    args = parser.parse_args()
    symbols = subprocess.check_output(["nm", "bin/system"], text=True)
    expecting_fault = args.expect in ("fault-divide", "fault-page") or args.expect in user_faults
    boundary = "kernel_halt" if expecting_fault or args.expect == "keyboard-check" else "kernel_keyboard_wait"
    address = next(line.split()[0] for line in symbols.splitlines()
                   if line.split()[-1] == boundary)
    expected_kernel = Path("bin/kernel.bin").read_bytes()
    loaded_kernel = Path("bin/kernel-loaded.bin")
    loaded_map = Path("bin/boot-memory-map.bin")
    # 清除上次取样，避免本次未到达入口时误用旧文件。物理 1 MiB 在过渡页表中
    # 恒等映射；在 head.S 第一条指令之前取样，尚未改页表/GDT/BSS 等内核内容。
    loaded_kernel.unlink(missing_ok=True)
    loaded_map.unlink(missing_ok=True)
    # 普通内核现在持续接收输入：到达等待入口才退出。键盘检查须收到四个字节才停机。
    Path("bin/smoke.rc").write_text(
        f"lb 0x100000\nlb 0x{address}\nc\n"
        f'writemem "bin/kernel-loaded.bin" 0x100000 {len(expected_kernel)}\n'
        'writemem "bin/boot-memory-map.bin" 0x4ff0 656\n'
        "delete 1\nc\nq\n")
    output = Path("bin/smoke-output.log")
    client = None
    try:
        with output.open("w") as stream:
            process = subprocess.Popen(
                ["bochs", "-q", "-f", "conf/smoke-bochsrc", "-rc", "bin/smoke.rc"],
                stdin=subprocess.DEVNULL, stdout=stream, stderr=subprocess.STDOUT,
            )
            try:
                deadline = time.monotonic() + 45
                if args.expect == "keyboard-check":
                    client = inject_keyboard(process, output, deadline)
                returncode = process.wait(timeout=max(0.01, deadline - time.monotonic()))
            except BaseException:
                process.kill()
                process.wait()
                raise
    except (subprocess.TimeoutExpired, TimeoutError, OSError, RuntimeError) as error:
        raise SystemExit(f"FAIL: {error}; inspect bin/smoke-output.log and bin/bochs-smoke.log") from error
    finally:
        if client:
            client.close()
    text = output.read_text(errors="replace")
    required = ["Loader copy OK: A20 verified; protected-mode sector copy",
                "Loader E820 complete: version=1, stride=20, capacity=32",
                "E820 handoff OK: version=1, entries=",
                "Boot memory layout OK: handoff, stack, VBE scratch, loader, temporary page tables",
                "VBE console ready", "Hello\tMiko\tOS!", "GDT/IDT/TSS ready",
                "Processor mode OK: IA-32e active, 64-bit code, CPL0; 4-level paging, canonical RIP/RSP",
                "Memory invariants OK", "Identity mapping removed; high-half mapping retained",
                "Physical page allocation/read-write/free OK",
                "8259A ready: vectors=0x20..0x2f, all IRQs masked",
                "PIT IRQ0 delivered twice; PIC EOI and IRETQ return OK",
                "Kernel thread created: pid=1",
                "init task is running, arg=0x000000000000000a",
                "Bootstrap task resumed: init yielded",
                "init task resumed after yielding",
                "Kernel thread exit: pid=1 code=1",
                "Kernel thread demo OK: yield, resume, exit=1; page reclaimed",
                "User mapping ready: code=0x800000 R-X, stack=0x9ff000 RW-NX; kernel supervisor"]
    if args.expect in user_faults:
        vector, error, label, cr2 = user_faults[args.expect]
        name = "page fault" if vector == 14 else "general protection"
        required += [f"EXCEPTION {vector} ({name}) ERROR={error:#018x}",
                     "User exception context: CS=0x002b SS=0x0033",
                     f"User protection check OK: {label}; CPL=3"]
        if cr2:
            required += [f"CR2={cr2}", "user=true", "reserved=false",
                         f"instruction_fetch={'true' if args.expect == 'fault-user-exec' else 'false'}"]
    else:
        required += ["SYSENTER ready: CS=0x8, dedicated kernel stack, 128 services",
                     "Hello World! %s %n",
                     "System call demo OK: SYSENTER/SYSEXIT64 returned to ring3; write=19, ENOSYS=-38",
                     "User mode OK: CS=0x2b SS=0x33 CPL=3, Rust stack/result verified; INT 0x80 used TSS.RSP0",
                     "User demo cleanup OK: ring0 restored, low mapping removed, page reclaimed",
                     "PS/2 keyboard ready: IRQ1, translated set 1 scan bytes",
                     "MikoOS Rust kernel ready"]
    if args.expect == "fault-divide":
        required += ["EXCEPTION 0 (divide error) ERROR=0x0000000000000000"]
    elif args.expect == "fault-page":
        required += ["EXCEPTION 14 (page fault) ERROR=0x0000000000000000",
                     "CR2=0xffff80000aa00000", "instruction_fetch=false"]
    elif args.expect == "memory-check":
        required += ["Page allocator checks OK: cross-word, reuse, fragmentation, exhaustion;",
                     "E820 input checks OK: header, overflow, overlaps, ordering, reserved ranges"]
    elif args.expect == "irq-check":
        required += ["IRQ context checks OK: GPRs, RFLAGS, RSP; software IRQ7/15 spurious paths"]
    elif args.expect == "keyboard-check":
        required += ["Keyboard queue checks OK: wraparound, FIFO, overflow",
                     "Keyboard IRQ1 checks OK: a make/break, b make/break; no drops/errors"]
    elif args.expect == "task-check":
        required += ["Task context checks OK: callee-saved GPRs, RSP, stack canaries; child IST return"]
    elif args.expect == "syscall-check":
        required += ["System call checks OK: GPRs, RSP, flags; number bounds, user ranges, length, UTF-8"]
    required += ["Breakpoint 1, 0x0000000000100000", f"Breakpoint 2, 0x{address}"]
    bochs_log = Path("bin/bochs-smoke.log").read_text(errors="replace")
    gui_reports = [line for line in bochs_log.splitlines() if ">>PANIC<<" in line]
    # 只容忍已确认的显示后端限制，不能把 CPU/设备 panic 一并忽略。
    unexpected_reports = [line for line in gui_reports if
                          "[RFB   ] >>PANIC<< 32 bpp graphics mode not supported yet" not in line]
    missing = [marker for marker in required if marker not in text]
    errors = [line for line in text.splitlines() if "PANIC:" in line] + unexpected_reports
    if not loaded_kernel.exists() or loaded_kernel.read_bytes() != expected_kernel:
        errors.append("loaded kernel bytes differ from bin/kernel.bin")
    # 原始交接字节与内核实际打印的解析结果交叉核对，避免只凭一条阶段日志判成功。
    raw = loaded_map.read_bytes() if loaded_map.exists() else b""
    if len(raw) != 656:
        errors.append("missing E820 handoff snapshot")
    else:
        magic, version, stride, count, capacity, flags = struct.unpack_from("<IHHHHI", raw)
        if (magic, version, stride, capacity, flags) != (0x30323845, 1, 20, 32, 1) or not 1 <= count <= 32:
            errors.append("invalid E820 handoff snapshot")
        else:
            if f"E820 handoff OK: version=1, entries={count}, stride=20;" not in text:
                errors.append("E820 header count differs from kernel")
            for index in range(count):
                base, length, kind = struct.unpack_from("<QQI", raw, 16 + index * 20)
                marker = f"Address:{base:#018x} Length:{length:#018x} Type:{kind}"
                if marker not in text:
                    errors.append(f"E820 descriptor {index} differs from kernel")
    exceptions = [line for line in text.splitlines() if line.startswith("EXCEPTION ")]
    if not expecting_fault and exceptions:
        errors.extend(exceptions)
    if expecting_fault and len(exceptions) != 1:
        errors.append(f"expected one exception, got {len(exceptions)}")
    if returncode or missing or errors:
        print(text[-6000:])
        raise SystemExit(f"FAIL: exit={returncode}, missing={missing}, errors={errors}")
    suffix = args.expect or "normal"
    copy_summary = f"Loader memory verified: {len(expected_kernel)} bytes at physical 0x100000"
    Path(f"bin/smoke-{suffix}.log").write_text(text + "\n" + copy_summary + "\n")
    Path(f"bin/bochs-{suffix}.log").write_text(bochs_log)
    start = text.index("VBE console ready")
    print(text[start:].split("Next at")[0].strip())
    if gui_reports:
        print("RFB limitation: 32 bpp display unavailable; CPU/MMIO/log verification only")
    print(copy_summary)
    print(f"PASS: Bochs {args.expect or 'normal'} boot reached {boundary}")


if __name__ == "__main__":
    main()
