"""Multi-turn repair loop: model writes a program, gets compiler/output feedback."""
import tempfile
from pathlib import Path

from .eris import compile_eris, run_binary
from .model import extract_code

SYSTEM_TMPL = """You are writing programs in Eris, a small compiled language you have not seen before.
Use ONLY the reference material below — do not guess syntax from other languages.
Reply with exactly one ```eris code block containing a complete program.

=== REFERENCE MATERIAL ===
{context}"""


def attempt_task(task, model, context, render_errors, max_turns=6):
    system = SYSTEM_TMPL.format(context=context)
    messages = [{"role": "user", "content": task["spec"]}]
    with tempfile.TemporaryDirectory() as td:
        workdir = Path(td)
        for turn in range(1, max_turns + 1):
            reply = model.complete(system, messages)
            source = extract_code(reply)
            binary, err = compile_eris(source, workdir)
            if err is None:
                stdout, code = run_binary(binary)
                if stdout == task["expected_stdout"] and code == 0:
                    return {"solved": True, "turns": turn}
                feedback = (
                    "The program compiled but produced the wrong output.\n"
                    f"Expected stdout:\n{task['expected_stdout']}\n"
                    f"Actual stdout:\n{stdout}\n(exit code {code})"
                )
            else:
                feedback = render_errors(err)
            messages.append({"role": "assistant", "content": reply})
            messages.append(
                {"role": "user", "content": feedback + "\nFix the program. Reply with one complete ```eris code block."}
            )
    return {"solved": False, "turns": max_turns}
