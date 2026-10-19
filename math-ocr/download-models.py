"""Explicit offline-deployment helper; never runs inference or reads user images."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--output", required=True)
parser.add_argument("--accept-source-size", action="store_true", help="Accept official GitHub v0.0.0, 178952787 bytes")
args = parser.parse_args()
if not args.accept_source_size:
    parser.error("Read README: explicit --accept-source-size is required before the 171 MiB download")
base = Path(__file__).resolve().parent
target = Path(args.output).resolve()
target.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="math-model-download-", dir=target) as stage:
    process = subprocess.Popen([sys.executable, "-I", "-c", (base / "worker.py").read_text(encoding="utf-8"), "download", str(target), (base / "models.json").read_text(encoding="utf-8"), stage], stdout=subprocess.PIPE)
    try:
        output, _ = process.communicate(timeout=310)
    except BaseException:
        process.kill()
        process.wait()
        raise
    print(output.decode("utf-8"))
    if process.returncode or not json.loads(output).get("ok"):
        raise SystemExit(1)
