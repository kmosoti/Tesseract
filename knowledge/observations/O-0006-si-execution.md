---
id: O-0006
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Executable SI language and Rust tooling

## Observed

SI Predict and Seek terms execute the kernel through a bounded parser and typed
interpreter. The intelligence diagnostic executes actions, journals feedback,
consolidates, and checks replay for three rounds. At seed 101 and zero noise,
frozen complex loss was 0.75; complex learning loss 0.10457648085732645; real
learning loss 0.10262198526909787. Both learners made three commits. These are
single-fixture diagnostics, not a statistical comparison.

Lean 4.24.0 checks abstract action composition and evidence-constructor laws.
Z3 checks proposal ranking and rejects deliberately suboptimal input. Neither
establishes numerical interpreter correctness. Rust xtasks replace knowledge
validation, GitHub configuration, issue seeding, and evidence reproduction scripts.
No governance mutation was performed while testing the tool migration.

Local checks passed: `cargo fmt --all --check`, locked workspace Clippy with
`-D warnings`, `cargo test --locked --workspace`, `cargo test --locked -p xtask`,
`cargo xtask knowledge`, `cargo xtask formal` (Lean on PATH), the release workspace
build, and the smoke comparison at `runs/si-smoke.json`. CLI execution of a Seek
term against a complex demo journal returned a program with revision/evidence
references. The reproduction xtask matched the first original frozen worker
record using the separate historical worktree; the full 300-result receipt was
not rerun. GitHub setup/issue mutation commands were ported but not executed.

## Inferred

The experiment supports executable symbolic control and persistent consolidation
under its fixed conditions. A real-valued representation suffices for these data.

## Hypothesis

A richer typed expression graph may improve inspectability and reusable reasoning.
The current language does not implement that graph.

## Unknown

Complex representation advantage, feedback-specific benefit beyond exhaustive
exploration, untouched-program generalization, and arbitrary controller recovery.
Remote governance and remote CI execution are separate from local checks.

## Relations

- *extends*: [O-0005](O-0005-symbolic-loop.md)
- *relates*: [D-0001](../decisions/D-0001-developmental-kernel.md)

## Revisit when

A frozen multi-seed protocol, richer language, or new solver/proof obligation is
introduced. Earlier Python command records remain historical evidence.
