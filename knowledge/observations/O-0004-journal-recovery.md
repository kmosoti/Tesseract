---
id: O-0004
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Journal recovery after a failed append

## Observed

The append path previously returned after a failed write and failed restoration
without preventing subsequent appends on the same session. The public repository
now exists; its `develop` branch contains the bundled implementation and its
`main` branch has an unrelated root. GitHub reported `main` unprotected.

On the revised implementation, `cargo fmt --all --check`,
`cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo test --locked --workspace`, `python3 scripts/validate_knowledge.py`,
and `git diff --check` passed. Tests use a read-only descriptor and injected
failures after a complete write; they do not simulate all storage failures.

## Inferred

When restoration fails, the file may no longer match the in-memory sequence and
head. Reusing that session can append records to an uncertain history. Rejecting
further writes and checkpoints until reopen bounds this failure mode.

## Hypothesis

Replaying after closing the session will recover a valid complete prefix or report
corruption, but an unsuccessful append can have reached disk as a complete record.

## Unknown

Real storage devices can fail in ways not represented by a read-only descriptor;
reopening does not guarantee recovery from complete corrupted records.

## Relations

- *extends*: [O-0002](O-0002-freeze-verification.md)
- *implements*: [D-0001](../decisions/D-0001-developmental-kernel.md)

## Revisit when

Additional fault-injection evidence or a different durability contract is available.
