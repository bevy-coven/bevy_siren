"""Drive `siren-tools` with the **real** CLAP text graph and time it — what `hostbench` could not measure.

    python3 drive_siren_tools.py <library-dir> <clap-dir>

Start-up is measured from spawn to the first reply, so it includes reading the index and loading the 501 MB text graph.
Every call is one round trip over the pipe, and `search_music` includes the real text encode.
"""
import json
import os
import subprocess
import sys
import time

LIBRARY, CLAP = sys.argv[1], sys.argv[2]
BINARY = os.path.join(os.path.dirname(os.path.abspath(__file__)), *[".."] * 6, "target", "release", "siren-tools")
VERSION = open(os.path.join(CLAP, "clap-export.txt")).read().strip()

started = time.perf_counter()
proc = subprocess.Popen(
    [BINARY, LIBRARY, "--model", os.path.join(CLAP, "clap-text.onnx"), "--vocab", os.path.join(CLAP, "tokenizer.json"),
     "--model-version", VERSION],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1,
)


def call(method, params, ident):
    t = time.perf_counter()
    proc.stdin.write(json.dumps({"jsonrpc": "2.0", "id": ident, "method": method, "params": params}) + "\n")
    proc.stdin.flush()
    reply = json.loads(proc.stdout.readline())
    return reply, (time.perf_counter() - t) * 1000.0


def tool(name, arguments, ident):
    reply, ms = call("tools/call", {"name": name, "arguments": arguments}, ident)
    return reply["result"], ms


reply, _ = call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}}, 1)
startup_ms = (time.perf_counter() - started) * 1000.0
print("start-up (spawn -> first reply, index read + 501 MB text graph loaded): %.0f ms" % startup_ms)

rows = []
for i, phrase in enumerate(["a short electronic beep", "a deep sonar ping underwater", "a glass being tapped"] * 2):
    result, ms = tool("search_music", {"description": phrase, "k": 3}, 10 + i)
    answer = result["structuredContent"]
    top = answer["results"][0]["asset_id"] if answer.get("results") else None
    rows.append(("search_music", phrase, ms, "isError" if result["isError"] else "ok"))
    first = answer["results"][0] if answer.get("results") else None
    if i == 0:
        print("first search returned: %s" % json.dumps({k: first[k] for k in ("asset_id", "score", "similarity")}))
anchor = tool("search_music", {"description": "a short electronic beep", "k": 1}, 30)[0]["structuredContent"]["results"][0]["asset_id"]
for name, args in [
    ("set_anchor", {"asset_id": anchor}),
    ("find_compatible_sfx", {"k": 6}),
    ("find_compatible_music", {"k": 3}),
    ("explain_match", {"asset_id": anchor, "anchor": anchor}),
]:
    result, ms = tool(name, args, 40)
    rows.append((name, json.dumps(args)[:44], ms, "isError" if result["isError"] else "ok"))

print("\n%-22s %-46s %9s  %s" % ("tool", "arguments", "round trip", ""))
for name, what, ms, state in rows:
    print("%-22s %-46s %7.2f ms  %s" % (name, what, ms, state))
proc.stdin.close()
proc.wait(timeout=30)
print("\nexit status %d; stderr:\n%s" % (proc.returncode, proc.stderr.read().strip()))
