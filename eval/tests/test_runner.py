import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from run_killtest import load_existing, write_rows_atomic


def test_write_rows_atomic_and_load_roundtrip(tmp_path):
    out = tmp_path / "results.csv"
    rows = [
        {"task": "t1", "condition": "plain", "sample": 0, "solved": True, "turns": 2},
        {"task": "t1", "condition": "structured", "sample": 0, "solved": False, "turns": 6},
    ]
    write_rows_atomic(out, rows)
    assert out.exists() and not out.with_suffix(".tmp").exists()
    loaded, done = load_existing(out)
    assert len(loaded) == 2
    assert loaded[0]["solved"] is True and loaded[0]["turns"] == 2
    assert loaded[1]["solved"] is False and loaded[1]["turns"] == 6
    assert loaded[0]["sample"] == 0 and loaded[1]["sample"] == 0
    assert ("t1", "plain", "0") in done
    assert ("t1", "structured", "0") in done


def test_load_existing_missing_file(tmp_path):
    rows, done = load_existing(tmp_path / "absent.csv")
    assert rows == [] and done == set()
