import json
import tempfile
from pathlib import Path

import pytest

from harness.eris import build_compiler, compile_eris, run_binary

TASKS_DIR = Path(__file__).resolve().parents[1] / "tasks"
TASKS = json.loads((TASKS_DIR / "tasks.json").read_text()) if (TASKS_DIR / "tasks.json").exists() else []


@pytest.mark.parametrize("task", TASKS, ids=lambda t: t["id"])
def test_reference_solution_matches_expected(task):
    build_compiler()
    ref = (TASKS_DIR / "refs" / f"{task['id']}.eris").read_text()
    with tempfile.TemporaryDirectory() as td:
        binary, err = compile_eris(ref, Path(td))
        assert err is None, f"reference for {task['id']} failed to compile:\n{err}"
        stdout, code = run_binary(binary)
    assert code == 0
    assert stdout == task["expected_stdout"]


def test_benchmark_has_twelve_tasks():
    assert len(TASKS) == 12
