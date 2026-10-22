#!/usr/bin/env bash
# Stack-sampling profiler for the headless fill workload (no sudo, no perf).
#
# Runs the release binary in `--profile` mode for one fill algorithm and
# samples its call stacks with `eu-stack` (elfutils, ptrace) every ~10 ms
# until the process exits. Each sample is one `eu-stack` snapshot of the main
# thread. Output: raw samples and an inclusive/self summary per function.
#
# Usage: docs/profiles/lab5-stack-sampler.sh <barycentric|scanline> <iterations> <outdir>
set -euo pipefail
mode="$1"; iterations="$2"; out="$3"
mkdir -p "$out"
raw="$out/lab5-stack-samples-$mode.txt"
summary="$out/lab5-stack-summary-$mode.txt"
bin="./target/release/rust-software-renderer"

"$bin" --benchmark-triangles --profile --profile-mode "$mode" --profile-iterations "$iterations" >"$out/lab5-profile-run-$mode.log" 2>&1 &
pid=$!
: >"$raw"
samples=0
sleep 0.3  # let the workload start before sampling
while kill -0 "$pid" 2>/dev/null; do
  if eu-stack -p "$pid" >>"$raw" 2>/dev/null; then
    samples=$((samples + 1))
    echo "--- sample $samples" >>"$raw"
  fi
  sleep 0.01
done
wait "$pid" || true

python3 - "$raw" "$summary" "$mode" "$samples" <<'PY'
import re, sys, collections
raw, summary, mode, samples = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
import subprocess
def demangle_all(symbols):
    # Rust v0/legacy mangling is understood by binutils c++filt.
    out = subprocess.run(["c++filt"], input="\n".join(symbols), capture_output=True, text=True).stdout.split("\n")
    return dict(zip(symbols, [re.sub(r"::h[0-9a-f]{16}$", "", o) for o in out]))
inclusive, selfc = collections.Counter(), collections.Counter()
stack = []
def flush():
    global stack
    if stack:
        selfc[stack[0]] += 1
        for f in set(stack): inclusive[f] += 1
    stack = []
for line in open(raw):
    if line.startswith("--- sample"):
        flush(); continue
    m = re.match(r"#\d+\s+0x[0-9a-f]+\s+(\S+)", line.strip())
    if m:
        stack.append(m.group(1))
flush()
names = demangle_all(sorted(set(list(selfc) + list(inclusive))))
selfc = collections.Counter({names[k]: v for k, v in selfc.items()})
inclusive = collections.Counter({names[k]: v for k, v in inclusive.items()})
total = sum(selfc.values())
with open(summary, "w") as f:
    f.write(f"mode={mode} samples={total} (eu-stack snapshots of the main thread, ~10 ms apart)\n")
    f.write("workload: --benchmark-triangles --profile on the fixed 207-triangle set (release build)\n\n")
    f.write("SELF (leaf frame)        samples   %\n")
    for fn, c in selfc.most_common(15):
        f.write(f"{c:8d} {100*c/total:6.1f}%  {fn}\n")
    f.write("\nINCLUSIVE (frame anywhere on the stack)\n")
    for fn, c in inclusive.most_common(20):
        f.write(f"{c:8d} {100*c/total:6.1f}%  {fn}\n")
print(open(summary).read())
PY
