#!/usr/bin/env python3
"""Extract kompress fine-tuning pairs from dogfeed JSONL data.

Reads data/loop-*.jsonl from PeetPedro/ultrawhale-dogfood and outputs
(original, compressed) pairs for training the kompress compression model.

Usage:
    python3 scripts/feedback-loop/extract-training-data.py \
        --output /tmp/kompress-train.jsonl \
        --min-compression-ratio 0.3 \
        --max-pairs 5000
"""
import argparse
import json
import sys
from pathlib import Path
from typing import Iterator


def load_jsonl(path: Path) -> Iterator[dict]:
    """Stream JSONL records from a file."""
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                yield json.loads(line)


def extract_pairs(records: Iterator[dict], min_ratio: float) -> list[dict]:
    """Extract (answer, compressed_answer) training pairs."""
    pairs = []
    for rec in records:
        answer = (rec.get("answer") or "").strip()
        compressed = (rec.get("compressed_answer") or "").strip()

        # Skip empty or invalid
        if not answer or not compressed:
            continue

        # Skip rows where compression made things longer
        if len(compressed) >= len(answer):
            continue

        # Skip if compression ratio too low (not enough compression)
        ratio = len(compressed) / len(answer)
        if ratio > min_ratio:
            continue

        pairs.append({
            "input": answer,
            "output": compressed,
            "topic": rec.get("topic", ""),
            "model": rec.get("model", ""),
            "role": rec.get("role", "unknown"),
        })
    return pairs


def main():
    parser = argparse.ArgumentParser(description="Extract kompress training data from dogfeed")
    parser.add_argument("--output", required=True, help="Output JSONL file")
    parser.add_argument("--source", help="Local JSONL file or directory (default: HF dataset)")
    parser.add_argument("--min-compression-ratio", type=float, default=0.5,
                        help="Max compressed/original length ratio (lower = more aggressive)")
    parser.add_argument("--max-pairs", type=int, default=10000,
                        help="Maximum training pairs to extract")
    args = parser.parse_args()

    records = []
    if args.source:
        src = Path(args.source)
        if src.is_dir():
            for f in sorted(src.glob("loop-*.jsonl")):
                records.extend(load_jsonl(f))
        else:
            records = list(load_jsonl(src))
    else:
        # Load from HF datasets library
        try:
            from datasets import load_dataset
            ds = load_dataset("PeetPedro/ultrawhale-dogfood", split="train", streaming=True)
            records = list(ds.take(args.max_pairs * 3))  # oversample, filter later
        except ImportError:
            print("datasets library not installed. Install with: pip install datasets", file=sys.stderr)
            print("Or use --source to point at a local JSONL file.", file=sys.stderr)
            sys.exit(1)
        except FileNotFoundError:
            print("No data files found on HF dataset yet (dogfeed loop may not have pushed).", file=sys.stderr)
            print("Use --source to point at a local JSONL file, or wait for the loop to generate data.", file=sys.stderr)
            sys.exit(1)

    pairs = extract_pairs(iter(records), args.min_compression_ratio)

    # Limit to max pairs
    if len(pairs) > args.max_pairs:
        pairs = pairs[:args.max_pairs]

    Path(args.output).parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w") as f:
        for pair in pairs:
            f.write(json.dumps(pair) + "\n")

    # Stats
    ratios = [len(p["output"]) / len(p["input"]) for p in pairs]
    avg_ratio = sum(ratios) / len(ratios) if ratios else 0
    roles = {}
    for p in pairs:
        roles[p["role"]] = roles.get(p["role"], 0) + 1

    print(f"Extracted {len(pairs)} training pairs")
    print(f"  Average compression ratio: {avg_ratio:.2f}")
    print(f"  Role distribution: {roles}")
    print(f"  Output: {args.output}")


if __name__ == "__main__":
    main()
