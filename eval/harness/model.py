"""Model client (Anthropic) plus a mock for tests."""
import re


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
        resp = self.client.messages.create(
            model=self.model_id,
            max_tokens=2048,
            system=system,
            messages=messages,
        )
        return resp.content[0].text


def extract_code(reply: str) -> str:
    blocks = re.findall(r"```(?:eris)?\s*\n(.*?)```", reply, re.S)
    return blocks[-1] if blocks else reply
