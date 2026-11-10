---
id: O-0009
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Working abstractions and experiment selection

## Observed

The nucleus derives typed conditional propositions from public observations,
including hypotheses that omit porosity or initial state. Cross-condition support
and distinct supporting objects are required; counterexamples are retained.
Four empirical grouping models supply disagreement-based experiment selection.
Working concepts and choices reconstruct from existing journal events.

The full-budget seed-114 diagnostic used 48 observations per controller and
reported committee Brier loss 0.6111872864440608 for coverage and
0.7368482905982925 for curiosity. No superiority assertion is encoded. The
exploration CLI performed 48 actual public experiments, accepted two numerical
revisions, rejected another at the unchanged gain threshold, and reopened with
matching state, concepts and next choice. That kernel's evaluation loss was
0.7224267946299178; it is not the committee diagnostic's prediction model.

## Inferred

The nucleus supplies working abstraction and causally different attention,
without establishing generally better exploration. Curiosity is a bounded
selection rule over rival hypotheses; it need not be useful on every world.

## Hypothesis

A broader hypothesis family and explicit coverage/disagreement tradeoff may
reduce overfocus. That requires a new evaluation protocol and untouched seeds.

## Unknown

Benefit beyond coverage, discovery of new predicates, transfer across worlds,
controller crash recovery, and correspondence between abstract Lean laws and
Rust execution. Supported working hypotheses are not validated universal laws.

## Relations

- *extends*: [O-0007](O-0007-english-adapter.md)
- *relates*: [D-0001](../decisions/D-0001-developmental-kernel.md)

## Revisit when

New predicates, memory policies, or a frozen multi-seed curiosity protocol are
introduced. Preserve negative results when changing the development instrument.

Executed local verification: locked workspace tests, nucleus and SI targeted
tests, exploration CLI exclusivity/replay checks, formatting, workspace Clippy
with warnings denied, knowledge validation, and `cargo xtask formal` with Lean
on PATH. Small-budget scheduling and malformed provenance/score regressions pass.
The Lean working-status reference proof was repaired and successfully checked;
these proofs do not establish Rust correspondence.
