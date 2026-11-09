#!/usr/bin/env python3
"""Re-run recorded workers and compare deterministic behavior hashes, not timings."""
import argparse
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=pathlib.Path)
    parser.add_argument("--binary", type=pathlib.Path, default=ROOT / "target/release/tesseract")
    args = parser.parse_args()
    receipt = json.loads(args.receipt.read_text())
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if revision != receipt["source_revision"]:
        raise SystemExit("Check out the recorded source revision in a separate worktree first")
    for recorded in receipt["results"]:
        behavior = recorded["behavior"]
        result = subprocess.run([str(args.binary), "worker", "--seed", str(behavior["seed"]),
                                 "--kind", behavior["kind"], "--protocol", "experiments/workshop-v1.json"],
                                cwd=ROOT, capture_output=True, text=True, check=True)
        replay = json.loads(result.stdout)
        if replay["behavior_hash"] != recorded["behavior_hash"]:
            raise SystemExit(f"Behavior mismatch: {behavior['seed']}/{behavior['kind']}")
    print(f"Reproduced {len(receipt['results'])} deterministic worker results; timing/RSS are not compared")
