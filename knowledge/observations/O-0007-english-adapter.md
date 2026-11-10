---
id: O-0007
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Bidirectional English adapter

## Observed

A Rust adapter translates four English sentence forms into validated SI Predict
and Seek terms, serializes canonical SI and English, and renders model Thought
values as estimates with revision/evidence references. CLI translation needs no
journal; ask/explain use saved state. Input bounds and unknown-word rejection
prevent silent truncation or guessed semantics.

Targeted SI tests include all 344 bounded term round trips, alternate forms,
rejected negation/out-of-range/injected requests, probability/provenance rendering,
and CLI translation. They pass locally, along with locked workspace Clippy.
Executed checks: `cargo test --locked --workspace`,
`cargo test --locked -p tesseract-si`, `cargo fmt --all --check`,
`cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo xtask knowledge`, `cargo xtask formal`, and `git diff --check`.
Ask and explain commands were exercised against a saved complex demo journal.

## Inferred

The supported English surface preserves the existing symbolic vocabulary without
requiring network services or neural-model dependencies. It introduces no new
learning capability or interpretation of numbered outcome states.

## Hypothesis

Additional grammatical forms may improve usability if they preserve explicit
state, action order, and rejection of ambiguous requests.

## Unknown

How broad user vocabulary should map into richer future SI concepts. The current
controlled grammar does not establish general language understanding.

## Relations

- *extends*: [O-0006](O-0006-si-execution.md)
- *relates*: [D-0001](../decisions/D-0001-developmental-kernel.md)

## Revisit when

SI gains new operators or outcome semantics, or unsupported user phrasing supplies
concrete evidence for extending the grammar.
