"""The two kill-test conditions: plain docs vs structured surface."""
import re
from pathlib import Path

from .eris import explain

REPO = Path(__file__).resolve().parents[2]


def plain_context() -> str:
    return (REPO / "LANGUAGE.md").read_text()


def structured_context() -> str:
    card = (REPO / "schemas" / "language-card.json").read_text()
    cookbook = (REPO / "schemas" / "cookbook.json").read_text()
    return (
        "Machine-readable language card (grammar, types, operators, builtins, error codes):\n"
        f"{card}\n\n"
        "Verified cookbook — every entry below compiles and its expected output is tested:\n"
        f"{cookbook}"
    )


def render_plain(err: str) -> str:
    return f"Compilation failed:\n{err}"


def render_structured(err: str) -> str:
    codes = sorted(set(re.findall(r"E\d{4}", err)))
    explains = "\n".join(explain(c) for c in codes)
    return (
        f"Compilation failed. Structured diagnostics:\n{err}\n\n"
        f"Documentation for each error code above:\n{explains}"
    )
