# Workshop v1 frozen evidence

[Raw receipt](receipt.json), generated from clean source commit
`62cfd5b3b21eb5b28bfcc7389d3de3e83487ccc6` with the unchanged
[registered protocol](../../experiments/workshop-v1.json).

Receipt SHA-256:
`b8e1fb2135ffb975f31da5f6b1447d47abc198851d19aabf947bb967d7c24a45`

Executable SHA-256:
`19b859c87a207711be295372a964a53fed7d1aea3b22c43cf1083087ecc3a67f`

Protocol digest:
`97a8cd4915e91213897a68317d03d00bb1a62d1d980a8c89ca3f25110b102846`

The receipt contains all 300 per-seed/representation results, seven comparisons,
behavioral hashes, timing/memory diagnostics, machine and toolchain metadata,
and limitations. See [the interpreted verdict](../../docs/findings.md).

## Reproduce

Do not modify the protocol after observing these results. Use a separate worktree
at the recorded source revision so the published knowledge records can continue
to evolve without changing this experiment's identity:

```bash
git worktree add --detach ../tesseract-workshop-v1 62cfd5b3b21eb5b28bfcc7389d3de3e83487ccc6
cd ../tesseract-workshop-v1
cargo build --locked --release --workspace
cargo run --locked --release -p tesseract-lab -- compare --tier benchmark --workers 2 --out runs/replication.json
cd -
cargo xtask reproduce ../tesseract-workshop-v1/runs/replication.json ../tesseract-workshop-v1/target/release/tesseract ../tesseract-workshop-v1
```

For comparison against the original receipt, pass its absolute path to
`cargo xtask reproduce RECEIPT BINARY SOURCE_DIR` from the current tooling checkout. It compares behavioral hashes;
observational timing, RSS, scheduling, and executable build-path differences are
not claimed to be bitwise reproducible. CPU and operating-system changes can
affect floating-point hashes; report such a deviation instead of rewriting this
receipt. The current tests and evidence establish behavior on the recorded Linux
machine, not the user's Fedora installation.
