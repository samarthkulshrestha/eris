#!/usr/bin/env python3
"""Kill-test: plain docs vs structured surface. See eval/KILLTEST.md."""
import argparse
import csv
import json
from pathlib import Path

from harness.conditions import (
    plain_context,
    render_plain,
    render_structured,
    structured_context,
)
from harness.eris import build_compiler
from harness.loop import attempt_task
from harness.model import ClaudeModel


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", default="claude-sonnet-5")
    ap.add_argument("--samples", type=int, default=3)
    ap.add_argument("--out", default="eval/results/killtest.csv")
    args = ap.parse_args()

    build_compiler()
    tasks = json.loads(Path("eval/tasks/tasks.json").read_text())
    conditions = {
        "plain": (plain_context(), render_plain),
        "structured": (structured_context(), render_structured),
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    rows = []
    for task in tasks:
        for cond, (context, render) in conditions.items():
            for sample in range(args.samples):
                model = ClaudeModel(args.model)
                result = attempt_task(task, model, context, render)
                row = {
                    "task": task["id"], "condition": cond, "sample": sample,
                    "solved": result["solved"], "turns": result["turns"],
                }
                rows.append(row)
                print(row, flush=True)
                # write incrementally so an aborted run keeps partial data
                with out.open("w", newline="") as f:
                    w = csv.DictWriter(f, fieldnames=rows[0].keys())
                    w.writeheader()
                    w.writerows(rows)

    for cond in conditions:
        sub = [r for r in rows if r["condition"] == cond]
        solved = sum(r["solved"] for r in sub)
        turns = [r["turns"] for r in sub if r["solved"]]
        mean_turns = sum(turns) / len(turns) if turns else float("nan")
        print(f"{cond}: solve rate {solved}/{len(sub)} "
              f"({100 * solved / len(sub):.0f}%), mean turns-to-solve {mean_turns:.2f}")


if __name__ == "__main__":
    main()
