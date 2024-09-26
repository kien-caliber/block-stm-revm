# Check the average, max, and min speedup of the latest criterion bench.

# $ cargo bench --bench mainnet -- --noplot
# $ python benches/report.py

# Ideally criterion would give us access to the benchmarked numbers via Rust
# API. They don't, so we must read from the output JSON files. They also don't
# expose the estimate types in Rust so we need to parse it manually. Picking
# Python with no error handling for dev speed and future plotting. We only use
# this for a quick report during performance tuning anyway.

import json
import os

CRITERION_PATH = "target/criterion"


def format_ms(ns):
    return str(round(ns / 1000000, 3))


def read_estimate(block, exec_type):
    with open(f"{CRITERION_PATH}/{block}/{exec_type}/new/estimates.json") as f:
        estimates = json.load(f)
        return (estimates["slope"] or estimates["mean"])["point_estimate"]


def to_blkno(name):
    if name.startswith("BLK"):
        return int(name.split()[1])
    else:
        return 0


for name in sorted(os.listdir(CRITERION_PATH), key=to_blkno):
    if name.startswith("BLK"):
        cases = ["S", "P0", "P8", "P16", "P32"]
        means = [read_estimate(name, c) for c in cases]
        print(name, " ".join(map(format_ms, means)))
