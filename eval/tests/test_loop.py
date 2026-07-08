import json
from pathlib import Path

from harness.loop import attempt_task
from harness.model import MockModel
from harness.conditions import render_plain

TASK = {
    "id": "answer",
    "spec": "Print the number 42 followed by a newline.",
    "expected_stdout": "42\n",
}

BROKEN = "```eris\nint main() { print_int(undeclared); println(); return 0; }\n```"
FIXED = "```eris\nint main() { print_int(42); println(); return 0; }\n```"


def test_loop_recovers_after_compile_error():
    model = MockModel([BROKEN, FIXED])
    result = attempt_task(TASK, model, context="(no docs needed)", render_errors=render_plain)
    assert result["solved"] is True
    assert result["turns"] == 2


def test_loop_gives_up_at_max_turns():
    model = MockModel([BROKEN] * 3)
    result = attempt_task(
        TASK, model, context="(no docs needed)", render_errors=render_plain, max_turns=3
    )
    assert result["solved"] is False
    assert result["turns"] == 3
