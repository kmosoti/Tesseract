# Tesseract

A CPU-first Rust laboratory for persistent developmental learning. Experience
updates working symbolic records; validated learning acquires durable transition
operators and bounded action programs. Bayesian, real-valued, and complex-valued
representations share public cognitive contracts.

**Status:** kernel v0.1 is implemented and independently reviewed locally. The
frozen 100-seed comparison supports persistent transition learning and bounded
macro reuse; complex channels did not outperform the real-valued candidate.
See [the measured verdict](docs/findings.md). Development is published on the
`develop` branch. No general-intelligence or unrestricted
concept-invention claim follows from these results.

## Run

Install Rust with rustup; the repository pins Rust 1.100 beta (dated 2026-10-18). Build and run from a
Git checkout. Repository tooling uses Rust xtasks.

Install Z3 on PATH before running tests. Install Lean via elan for
`cargo xtask formal`; `lean-toolchain` selects Lean 4.24.0.

```bash
cargo test --locked --workspace
cargo build --locked --release --workspace
mkdir -p runs
cargo run --locked --release -p tesseract-lab -- demo --journal runs/demo.jsonl
cargo run --locked --release -p tesseract-lab -- replay runs/demo.jsonl --checkpoint runs/demo.checkpoint.json
cargo run --locked --release -p tesseract-lab -- compare --tier smoke --workers 2 --out runs/smoke.json
```

Journal and evidence paths are exclusive; choose a new path for a new run. Use
`--kind real` or `--kind complex` with `demo` to exercise the matrix candidates.
The coordinator runs isolated workers, capped at four. Each Linux process is
limited to 1 GiB virtual address space. No GPU, model download, LLM, or network
service is required at runtime.

Translate English goals into SI or explain SI in English:

```bash
cargo run --locked -p tesseract-si -- translate --to-si 'How can a porous object go from state 0 to state 2?'
cargo run --locked -p tesseract-si -- translate --to-english 'predict solid 1 heat cool'
```

See the [supported English grammar and saved-state commands](docs/symbolic-language-experiment.md#english-adapter).

## What this tests

The workshop has noisy public observations and hidden, seeded material rules.
Learned processes must predict unseen action triples on new objects and survive
actual journal reopen. Automatic validation keeps held-out objects separate from
training, checks retained competence, and rejects stale or invalid candidates.
Learned macros prioritize tightly bounded proposal search; a disabled-library
ablation tests that contribution separately.

Complex mathematics is a candidate substrate. Classical order effects and useful
learning do not establish quantum advantage. Matrix candidates start from an
empirical classical embedding and have different coordinate/storage costs.

- [Architecture and precise limitations](docs/architecture.md)
- [Protocol, controls, metrics, and gates](docs/methodology.md)
- [Developmental roadmap](docs/roadmap.md)
- [Symbolic language exploration](docs/symbolic-language-experiment.md)
- [Frozen results and reproduction](evidence/workshop-v1/README.md)
- [Linked knowledge graph](knowledge/README.md)
- [Contributor workflow](CONTRIBUTING.md) and [agent agreement](AGENTS.md)
- [Publication status](docs/publication.md)

Project knowledge is revisable Markdown with linked D-/R-/O- records. Runtime
memory is a separate versioned event journal with provenance and rollback.

MIT License. Copyright 2026 Kennedy Mosoti.

Experimental [working concepts and curiosity](docs/nucleus.md) provide a symbolic
nucleus over public experience. The [optional Burn GPU probe](docs/gpu-probe.md)
measures acceleration separately from the validated CPU runtime.
