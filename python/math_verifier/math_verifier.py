import json
import os
import re
import subprocess
import sys
import time

SANDBOX_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sandbox.py")
SANDBOX_TIMEOUT_SECONDS = 8
NUMBER_RE = re.compile(r"-?\d+(?:\.\d+)?")


def claim_number(claim):
    matches = NUMBER_RE.findall(claim or "")
    if not matches:
        return None
    try:
        return float(matches[-1])
    except ValueError:
        return None


def close_enough(a, b):
    return abs(a - b) <= max(1e-9, 1e-6 * abs(b))


def run_sandbox(script):
    started = time.time()
    try:
        proc = subprocess.run(
            [sys.executable, SANDBOX_PATH],
            input=script,
            capture_output=True,
            text=True,
            timeout=SANDBOX_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        return {
            "status": "timeout",
            "stdout": "",
            "value": None,
            "note": "script exceeded the execution limit",
            "duration_ms": int((time.time() - started) * 1000),
        }

    duration_ms = int((time.time() - started) * 1000)
    out = (proc.stdout or "").strip()
    if not out:
        return {
            "status": "runtime_error",
            "stdout": (proc.stderr or "sandbox produced no output")[-2000:],
            "value": None,
            "duration_ms": duration_ms,
        }
    try:
        payload = json.loads(out.splitlines()[-1])
    except json.JSONDecodeError:
        return {
            "status": "runtime_error",
            "stdout": out[-2000:],
            "value": None,
            "duration_ms": duration_ms,
        }
    payload["duration_ms"] = duration_ms
    return payload


def handle_verify(params):
    claim = params.get("claim", "")
    script = params.get("script", "")
    result = run_sandbox(script)

    if result["status"] == "ok":
        expected = claim_number(claim)
        value = result.get("value")
        if expected is not None and value is not None and not close_enough(expected, value):
            result["status"] = "mismatch"
            result["note"] = "claim states {}, script computed {}".format(expected, value)
        else:
            result["status"] = "verified"
    return result


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError as exc:
            response = {"jsonrpc": "2.0", "id": None,
                        "error": {"code": -32700, "message": str(exc)}}
            sys.stdout.write(json.dumps(response) + "\n")
            sys.stdout.flush()
            continue

        request_id = request.get("id")
        method = request.get("method")
        params = request.get("params", {})
        try:
            if method == "verify":
                response = {"jsonrpc": "2.0", "id": request_id,
                            "result": handle_verify(params)}
            else:
                response = {"jsonrpc": "2.0", "id": request_id,
                            "error": {"code": -32601,
                                      "message": "unknown method: {}".format(method)}}
        except Exception as exc:  # keep the service alive for the harness
            response = {"jsonrpc": "2.0", "id": request_id,
                        "error": {"code": -32000, "message": str(exc)}}
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
