#!/usr/bin/env python3
"""Kill-test: plain docs vs structured surface. See eval/KILLTEST.md."""
import argparse
import csv
import json
import os
from pathlib import Path

from harness.conditions import (
    plain_context,
    render_plain,
    render_structured,
    structured_context,
)
from harness.eris import build_compiler
from harness.loop import attempt_task
from harness.model import ClaudeCodeModel, ClaudeModel

FIELDNAMES = ["task", "condition", "sample", "solved", "turns"]


def write_rows_atomic(out: Path, rows: list[dict]) -> None:
    tmp = out.with_suffix(".tmp")
    with tmp.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=FIELDNAMES)
        w.writeheader()
        w.writerows(rows)
    os.replace(tmp, out)


def load_existing(out: Path) -> tuple[list[dict], set[tuple]]:
    if not out.exists():
        return [], set()
    with out.open(newline="") as f:
        rows = list(csv.DictReader(f))
    done = {(r["task"], r["condition"], r["sample"]) for r in rows}
    for r in rows:  # CSV round-trips everything as strings; restore native types
        r["solved"] = r["solved"] == "True"
        r["turns"] = int(r["turns"])
        r["sample"] = int(r["sample"])
    return rows, done


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", default="claude-sonnet-5")
    ap.add_argument("--samples", type=int, default=3)
    ap.add_argument("--out", default="eval/results/killtest.csv")
    ap.add_argument("--backend", choices=["api", "claude-code"], default="api")
    args = ap.parse_args()

    build_compiler()
    tasks = json.loads(Path("eval/tasks/tasks.json").read_text())
    conditions = {
        "plain": (plain_context(), render_plain),
        "structured": (structured_context(), render_structured),
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    rows, done = load_existing(out)
    if done:
        print(f"resuming: {len(done)} attempts already recorded", flush=True)
    for task in tasks:
        for cond, (context, render) in conditions.items():
            for sample in range(args.samples):
                if (task["id"], cond, str(sample)) in done:
                    continue
                model = ClaudeCodeModel(args.model) if args.backend == "claude-code" else ClaudeModel(args.model)
                result = attempt_task(task, model, context, render)
                row = {
                    "task": task["id"], "condition": cond, "sample": sample,
                    "solved": result["solved"], "turns": result["turns"],
                }
                rows.append(row)
                print({**row, "backend": args.backend}, flush=True)
                # write atomically so an aborted run keeps partial data
                write_rows_atomic(out, rows)

    for cond in conditions:
        sub = [r for r in rows if r["condition"] == cond]
        solved = sum(r["solved"] for r in sub)
        turns = [r["turns"] for r in sub if r["solved"]]
        mean_turns = sum(turns) / len(turns) if turns else float("nan")
        print(f"{cond}: solve rate {solved}/{len(sub)} "
              f"({100 * solved / len(sub):.0f}%), mean turns-to-solve {mean_turns:.2f}")


if __name__ == "__main__":
    main()
