"""Model client (Anthropic) plus a mock for tests."""
import json
import os
import re
import subprocess
import time

# Narrow phrases only: bare "limit"/"usage" would misclassify e.g. the CLI's
# "Usage: claude [options]" argv-error output as a usage cap and retry for hours.
USAGE_LIMIT_RE = re.compile(
    r"usage limit|rate.?limit|limit reached|too many requests|overloaded"
    r"|out of extended usage|hit your limit",
    re.I,
)
MAX_USAGE_RETRIES = 24
USAGE_RETRY_SLEEP_SECONDS = 900


class MockModel:
    def __init__(self, replies):
        self.replies = list(replies)

    def complete(self, system, messages):
        return self.replies.pop(0)


class ClaudeModel:
    def __init__(self, model_id: str):
        import anthropic

        self.client = anthropic.Anthropic()
        self.model_id = model_id

    def complete(self, system, messages):
        import anthropic

        for attempt in range(4):
            try:
                resp = self.client.messages.create(
                    model=self.model_id,
                    max_tokens=2048,
                    system=system,
                    messages=messages,
                )
                return resp.content[0].text
            except (
                anthropic.RateLimitError,
                anthropic.APIConnectionError,
                anthropic.InternalServerError,
            ):
                if attempt == 3:
                    raise
                time.sleep(10 * 2**attempt)


def flatten_messages(messages) -> str:
    labels = {"user": "[User]", "assistant": "[Your previous reply]"}
    parts = [f"{labels[m['role']]}\n{m['content']}" for m in messages]
    return "\n\n".join(parts)


class ClaudeCodeModel:
    """Drives `claude -p` headless mode, billed against a Claude Max subscription
    rather than the Anthropic API. Same interface as ClaudeModel."""

    def __init__(self, model_id: str):
        self.model_id = model_id

    def complete(self, system, messages):
        prompt = flatten_messages(messages)
        argv = [
            "claude", "-p",
            "--model", self.model_id,
            "--system-prompt", system,
            "--output-format", "json",
            "--no-session-persistence",
            "--tools=",  # empty tool list; "--tools" alone is variadic and would swallow prompt
            prompt,
        ]
        # ponytail: drop ANTHROPIC_API_KEY so auth falls back to the subscription
        # login instead of the (credit-less) API key — verified empirically that
        # its presence flips `claude auth status` to apiKeySource=ANTHROPIC_API_KEY.
        env = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}

        for attempt in range(1, MAX_USAGE_RETRIES + 1):
            proc = subprocess.run(argv, capture_output=True, text=True, timeout=300, env=env)
            data = None
            if proc.stdout:
                try:
                    data = json.loads(proc.stdout)
                except json.JSONDecodeError:
                    data = None

            failed = proc.returncode != 0 or (data is not None and data.get("is_error"))
            if not failed and data is not None:
                return data["result"]

            haystack = proc.stderr + proc.stdout
            if USAGE_LIMIT_RE.search(haystack):
                if attempt == MAX_USAGE_RETRIES:
                    raise RuntimeError(
                        f"claude-code backend: usage-limited after {MAX_USAGE_RETRIES} retries. "
                        f"stderr: {proc.stderr}"
                    )
                print(
                    f"claude-code backend: usage-limited, sleeping 15m (retry {attempt}/{MAX_USAGE_RETRIES})",
                    flush=True,
                )
                time.sleep(USAGE_RETRY_SLEEP_SECONDS)
                continue

            raise RuntimeError(
                f"claude-code backend failed: stderr={proc.stderr!r} stdout={proc.stdout[:500]!r}"
            )


def extract_code(reply: str) -> str:
    blocks = re.findall(r"```(?:eris)?\s*\n(.*?)```", reply, re.S)
    return blocks[-1] if blocks else reply
