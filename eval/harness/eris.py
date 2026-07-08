"""Compile-and-run wrapper around the eris CLI."""
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ERIS_BIN = REPO / "target" / "debug" / "eris"


def build_compiler():
    subprocess.run(
        ["cargo", "build", "-p", "eris-cli"],
        cwd=REPO, check=True, capture_output=True,
    )


def compile_eris(source: str, workdir: Path):
    """Returns (binary_path, None) on success or (None, error_text) on failure."""
    src = workdir / "prog.eris"
    src.write_text(source)
    proc = subprocess.run(
        [str(ERIS_BIN), "--format", "json", "-o", str(workdir), str(src)],
        capture_output=True, text=True, timeout=60,
    )
    if proc.returncode != 0:
        return None, (proc.stderr + proc.stdout).strip()
    return workdir / "prog", None


def run_binary(binary: Path):
    proc = subprocess.run(
        [str(binary)], capture_output=True, text=True, timeout=10,
    )
    return proc.stdout, proc.returncode


def explain(code: str) -> str:
    proc = subprocess.run(
        [str(ERIS_BIN), "--explain", code, "--format", "json"],
        capture_output=True, text=True, timeout=30,
    )
    return proc.stdout
