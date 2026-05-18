#!/usr/bin/env python3
import re
import sys
from pathlib import Path


def extract_undetected(rust_output_path):
    rx1 = re.compile(r"fault on wire\s+(\S+)\s+s-a-(\d)")
    rx2 = re.compile(r"^(\S+)\s*/\s*([01])$")
    rx3 = re.compile(r"^(\S+)\s+s-a-(\d)$")
    results = []
    with open(rust_output_path, 'r', encoding='utf-8', errors='ignore') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            m = rx1.search(line)
            if m:
                name, bit = m.group(1), m.group(2)
                results.append(f"{name} /{bit}")
                continue
            m = rx2.match(line)
            if m:
                name, bit = m.group(1), m.group(2)
                results.append(f"{name} /{bit}")
                continue
            m = rx3.match(line)
            if m:
                name, bit = m.group(1), m.group(2)
                results.append(f"{name} /{bit}")
                continue
    return results


def main():
    if len(sys.argv) < 2:
        print("Usage: rust_to_atlanta_faults.py <circuit_number>")
        sys.exit(2)
    num = sys.argv[1]
    cwd = Path(__file__).resolve().parents[1]
    results_dir = cwd / 'results'
    rust_out = results_dir / f"rust_output_{num}.txt"
    if not rust_out.exists():
        print(f"Rust output not found: {rust_out}")
        sys.exit(3)
    faults = extract_undetected(rust_out)
    out_file = results_dir / f"c{num}_rust_undetected_for_atlanta.flt"
    if not faults:
        print("No undetected faults found in rust output.")
        out_file.write_text("", encoding='utf-8')
        sys.exit(0)

    # remove duplicates while preserving order
    seen = set()
    unique = []
    for fline in faults:
        if fline not in seen:
            seen.add(fline)
            unique.append(fline)

    out_file.write_text('\n'.join(unique) + '\n', encoding='utf-8')
    print(f"Wrote {len(faults)} faults to {out_file}")


if __name__ == '__main__':
    main()
