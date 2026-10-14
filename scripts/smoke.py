#!/usr/bin/env python3
"""实现完成后验证真实软盘启动；在最终停机函数处断点并退出 Bochs。"""
import argparse
from pathlib import Path
import subprocess


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expect", choices=["fault-divide", "fault-page", "memory-check"])
    args = parser.parse_args()
    symbols = subprocess.check_output(["nm", "bin/system"], text=True)
    address = next(line.split()[0] for line in symbols.splitlines()
                   if line.split()[-1] == "kernel_halt")
    # CPU 真正执行到 Rust 停机边界才退出，不以启动后的固定延迟判定成功。
    Path("bin/smoke.rc").write_text(f"lb 0x{address}\nc\nq\n")
    output = Path("bin/smoke-output.log")
    try:
        with output.open("w") as stream:
            result = subprocess.run(
                ["bochs", "-q", "-f", "conf/smoke-bochsrc", "-rc", "bin/smoke.rc"],
                stdin=subprocess.DEVNULL, stdout=stream, stderr=subprocess.STDOUT,
                timeout=45, check=False,
            )
    except subprocess.TimeoutExpired:
        raise SystemExit("FAIL: Bochs did not reach kernel_halt within 45s; inspect bin/smoke-output.log and bin/bochs-smoke.log")
    text = output.read_text(errors="replace")
    required = ["VBE console ready", "Hello\tMiko\tOS!", "GDT/IDT/TSS ready",
                "Memory invariants OK", "Identity mapping removed; high-half mapping retained",
                "Physical page allocation/read-write/free OK",
                "MikoOS Rust kernel ready"]
    if args.expect == "fault-divide":
        required += ["EXCEPTION 0 (divide error) ERROR=0x0000000000000000"]
    elif args.expect == "fault-page":
        required += ["EXCEPTION 14 (page fault) ERROR=0x0000000000000000",
                     "CR2=0xffff80000aa00000", "instruction_fetch=false"]
    elif args.expect == "memory-check":
        required += ["Page allocator checks OK: cross-word, reuse, fragmentation, exhaustion;"]
    required += [f"Breakpoint 1, 0x{address}"]
    bochs_log = Path("bin/bochs-smoke.log").read_text(errors="replace")
    gui_reports = [line for line in bochs_log.splitlines() if ">>PANIC<<" in line]
    # 只容忍已确认的显示后端限制，不能把 CPU/设备 panic 一并忽略。
    unexpected_reports = [line for line in gui_reports if
                          "[RFB   ] >>PANIC<< 32 bpp graphics mode not supported yet" not in line]
    missing = [marker for marker in required if marker not in text]
    errors = [line for line in text.splitlines() if "PANIC:" in line] + unexpected_reports
    exceptions = [line for line in text.splitlines() if line.startswith("EXCEPTION ")]
    expecting_fault = args.expect in ("fault-divide", "fault-page")
    if not expecting_fault and exceptions:
        errors.extend(exceptions)
    if expecting_fault and len(exceptions) != 1:
        errors.append(f"expected one exception, got {len(exceptions)}")
    if result.returncode or missing or errors:
        print(text[-6000:])
        raise SystemExit(f"FAIL: exit={result.returncode}, missing={missing}, errors={errors}")
    suffix = args.expect or "normal"
    Path(f"bin/smoke-{suffix}.log").write_text(text)
    Path(f"bin/bochs-{suffix}.log").write_text(bochs_log)
    start = text.index("VBE console ready")
    print(text[start:].split("Next at")[0].strip())
    if gui_reports:
        print("RFB limitation: 32 bpp display unavailable; CPU/MMIO/log verification only")
    print(f"PASS: Bochs {args.expect or 'normal'} boot reached kernel_halt")


if __name__ == "__main__":
    main()
