import tempfile
from pathlib import Path

from harness.eris import build_compiler, compile_eris, run_binary

GOOD = """int main() {
    print_int(41 + 1);
    println();
    return 0;
}
"""

BAD = """int main() {
    return undeclared_name;
}
"""


def test_good_program_compiles_and_runs():
    build_compiler()
    with tempfile.TemporaryDirectory() as td:
        binary, err = compile_eris(GOOD, Path(td))
        assert err is None
        stdout, code = run_binary(binary)
    assert stdout == "42\n"
    assert code == 0


def test_bad_program_returns_structured_error():
    build_compiler()
    with tempfile.TemporaryDirectory() as td:
        binary, err = compile_eris(BAD, Path(td))
    assert binary is None
    assert "E0201" in err  # undeclared variable, JSON envelope
