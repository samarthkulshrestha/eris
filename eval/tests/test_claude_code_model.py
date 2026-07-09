import json

from harness.model import ClaudeCodeModel


def fake_result(result="42\n", is_error=False):
    return json.dumps({"type": "result", "subtype": "success", "is_error": is_error, "result": result})


class FakeCompleted:
    def __init__(self, stdout="", stderr="", returncode=0):
        self.stdout = stdout
        self.stderr = stderr
        self.returncode = returncode


def test_builds_expected_argv(monkeypatch):
    captured = {}

    def fake_run(argv, **kwargs):
        captured["argv"] = argv
        return FakeCompleted(stdout=fake_result("OK"))

    monkeypatch.setattr("harness.model.subprocess.run", fake_run)
    model = ClaudeCodeModel("claude-sonnet-5")
    model.complete("sys prompt", [{"role": "user", "content": "hi"}])

    argv = captured["argv"]
    assert "-p" in argv
    assert "claude-sonnet-5" in argv
    assert "--output-format" in argv and "json" in argv
    assert "[User]\nhi" in argv


def test_returns_result_text_on_success(monkeypatch):
    monkeypatch.setattr(
        "harness.model.subprocess.run",
        lambda argv, **kwargs: FakeCompleted(stdout=fake_result("the answer")),
    )
    model = ClaudeCodeModel("claude-sonnet-5")
    reply = model.complete("sys", [{"role": "user", "content": "hi"}])
    assert reply == "the answer"


def test_flattens_multiple_messages(monkeypatch):
    captured = {}

    def fake_run(argv, **kwargs):
        captured["argv"] = argv
        return FakeCompleted(stdout=fake_result("ok"))

    monkeypatch.setattr("harness.model.subprocess.run", fake_run)
    model = ClaudeCodeModel("claude-sonnet-5")
    model.complete(
        "sys",
        [
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": "second"},
        ],
    )
    prompt = captured["argv"][-1]
    assert prompt == "[User]\nfirst\n\n[Your previous reply]\nsecond"


def test_retries_on_usage_limit_then_succeeds(monkeypatch):
    monkeypatch.setattr("harness.model.time.sleep", lambda seconds: None)
    calls = {"n": 0}

    def fake_run(argv, **kwargs):
        calls["n"] += 1
        if calls["n"] == 1:
            return FakeCompleted(stdout=fake_result("usage limit reached", is_error=True), returncode=1)
        return FakeCompleted(stdout=fake_result("done"))

    monkeypatch.setattr("harness.model.subprocess.run", fake_run)
    model = ClaudeCodeModel("claude-sonnet-5")
    reply = model.complete("sys", [{"role": "user", "content": "hi"}])
    assert reply == "done"
    assert calls["n"] == 2


def test_raises_after_max_retries_on_usage_limit(monkeypatch):
    monkeypatch.setattr("harness.model.time.sleep", lambda seconds: None)
    calls = {"n": 0}

    def fake_run(argv, **kwargs):
        calls["n"] += 1
        return FakeCompleted(stdout=fake_result("rate limit exceeded", is_error=True), returncode=1)

    monkeypatch.setattr("harness.model.subprocess.run", fake_run)
    model = ClaudeCodeModel("claude-sonnet-5")
    try:
        model.complete("sys", [{"role": "user", "content": "hi"}])
        assert False, "expected RuntimeError"
    except RuntimeError:
        pass
    assert calls["n"] == 24


def test_raises_immediately_on_non_usage_error(monkeypatch):
    monkeypatch.setattr(
        "harness.model.subprocess.run",
        lambda argv, **kwargs: FakeCompleted(stdout="", stderr="boom: bad flag", returncode=1),
    )
    model = ClaudeCodeModel("claude-sonnet-5")
    try:
        model.complete("sys", [{"role": "user", "content": "hi"}])
        assert False, "expected RuntimeError"
    except RuntimeError as e:
        assert "boom: bad flag" in str(e)
