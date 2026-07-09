from harness.loop import attempt_task
from harness.model import ClaudeModel, MockModel
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


HANGING = "```eris\nint main() { mut int i := 0; while i < 1 { i = i - 1; } return 0; }\n```"


def test_loop_survives_hanging_program(monkeypatch):
    import functools
    import harness.loop as loop_mod
    from harness.eris import run_binary

    monkeypatch.setattr(loop_mod, "run_binary", functools.partial(run_binary, timeout=1))
    model = MockModel([HANGING])
    result = attempt_task(
        TASK, model, context="(no docs needed)", render_errors=render_plain, max_turns=1
    )
    assert result["solved"] is False
    assert result["turns"] == 1


def test_claude_model_retries_transient_errors(monkeypatch):
    import anthropic
    import httpx
    import harness.model as model_mod

    monkeypatch.setattr(model_mod.time, "sleep", lambda seconds: None)

    request = httpx.Request("POST", "https://api.anthropic.com")
    calls = {"n": 0}

    class FakeContentBlock:
        text = "42\n"

    class FakeResponse:
        content = [FakeContentBlock()]

    class FakeMessages:
        def create(self, **kwargs):
            calls["n"] += 1
            if calls["n"] < 3:
                raise anthropic.APIConnectionError(request=request)
            return FakeResponse()

    class FakeClient:
        messages = FakeMessages()

    model = ClaudeModel.__new__(ClaudeModel)
    model.client = FakeClient()
    model.model_id = "claude-test"

    result = model.complete("system", [{"role": "user", "content": "hi"}])

    assert result == "42\n"
    assert calls["n"] == 3
