import contextlib
import io
import json
import resource
import sys

ALLOWED_MODULES = {"math", "decimal", "fractions", "statistics", "datetime", "itertools", "functools"}

SAFE_NAMES = [
    "abs", "all", "any", "bool", "dict", "divmod", "enumerate", "filter",
    "float", "format", "frozenset", "int", "len", "list", "map", "max",
    "min", "pow", "print", "range", "reversed", "round", "set", "sorted",
    "str", "sum", "tuple", "zip",
]


def guarded_import(name, *args, **kwargs):
    root = name.split(".")[0]
    if root not in ALLOWED_MODULES:
        raise ImportError("module '{}' is not allowed in verification scripts".format(name))
    return __import__(name, *args, **kwargs)


def emit(status, stdout="", value=None):
    print(json.dumps({"status": status, "stdout": stdout, "value": value}))


def main():
    script = sys.stdin.read()

    resource.setrlimit(resource.RLIMIT_CPU, (5, 5))
    resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))

    import builtins as _builtins
    safe = {name: getattr(_builtins, name) for name in SAFE_NAMES if hasattr(_builtins, name)}
    safe["__import__"] = guarded_import

    env = {"__builtins__": safe}
    captured = io.StringIO()
    try:
        with contextlib.redirect_stdout(captured):
            exec(compile(script, "<verify>", "exec"), env)
    except SyntaxError as exc:
        emit("syntax_error", "{}: {}".format(type(exc).__name__, exc))
        return
    except Exception as exc:
        emit("runtime_error", "{}: {}".format(type(exc).__name__, exc))
        return

    stdout = captured.getvalue()
    lines = [line for line in stdout.splitlines() if line.strip()]
    if not lines:
        emit("non_numeric", stdout)
        return
    try:
        value = float(lines[-1])
    except ValueError:
        emit("non_numeric", stdout)
        return
    emit("ok", stdout, value)


if __name__ == "__main__":
    main()
