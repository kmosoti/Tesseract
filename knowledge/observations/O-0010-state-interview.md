---
id: O-0010
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Question-only state and evidence-driven learning

## Observed

A local interview ran at source revision
`0435a1d6208963175ced10e13f561222f679150c`, using the complex kernel and the
noiseless seed-115 workshop. Seven questions were repeated at seven snapshots:
identity, concepts, curiosity, three Cool predictions (nonporous initial 0 and 3,
porous initial 3), and a nonporous state-0-to-state-2 goal. Forty-nine attempts
included seven identity-question parser rejections. All question batteries left
both the full kernel digest and journal bytes unchanged.

Teaching supplied two public Cool observations at each initial state 0, 1 and 2,
first for nonporous objects, then porous objects. Initial state 3 was excluded
from this initial curriculum. Six examples produced one supported working
abstraction at numerical revision 0; twelve examples produced two abstractions
and eight contested hypotheses, still at revision 0. The supported abstractions
predicted Cool outcome 2 for nonporous objects and outcome 0 for porous objects,
independent of initial state. These were working hypotheses, not validated laws.

A four-evaluation candidate passed the unchanged default validation policy on
96 separate validation objects. Revision 1 predicted outcome 2 with probability
0.75 for nonporous initial state 0, but remained uniform for initial state 3.
Sixteen subsequent disagreement-selected experiments all used Cool and included
initial state 3. Working evidence changed before numerical revision changed.

A second four-evaluation candidate passed validation. At revision 2, nonporous
initial-state-0 and initial-state-3 Cool predictions assigned 0.85 to outcome 2;
porous initial-state-3 Cool assigned 0.8125 to outcome 0. There were 28 observed
objects, two supported abstractions, ten contested hypotheses and eight acquired
single-action processes. No multistep process acquisition was demonstrated.

The two candidate validation losses were 0.5630787037037037 and 0.5128125, from
a uniform-parent loss of 0.75. The same validation set was reused; these are
promotion diagnostics, not an independent generalization comparison.

Actual journal reopen preserved the complete state hash and answers. Separate
repository CLI replay and prediction calls matched the interview's final state
and initial-state-3 forecast. The local run retains the Rust instrument, frozen
protocol, per-phase full kernel snapshots, complete report, journal, and hashes.
Luna independently reviewed the saved artifacts and recorded observations; it
did not generate simulated user dialogue or claim subjective mental states.

## Inferred

Questions alone currently do not teach Tesseract. Public observations update
working hypotheses; accepted consolidation changes numerical competence. The
nucleus's abstraction did not provide zero-shot numerical prediction for initial
state 3: the numerical predictor remained uniform until examples of that state
were observed and consolidated. This is a concrete boundary between working
symbolic abstraction and validated numerical prediction.

## Hypothesis

Validated incorporation of supported abstractions into candidate operators may
allow transfer to unobserved initial states. That mechanism and its controls are
not established by this interview. Disagreement selection may overfocus known
action programs while all structural models agree uniformly on unseen programs.

## Unknown

Performance on other worlds/noise, a matched coverage control for this curriculum,
transfer to still-unobserved cases, broader natural-language understanding and
whether abstraction-to-operator integration would improve predictions safely.

## Relations

- *extends*: [O-0009](O-0009-working-nucleus.md)
- *relates*: [O-0007](O-0007-english-adapter.md)

## Revisit when

A new interview changes the curriculum or supplies evidence about transfer, or a
validated abstraction-to-prediction mechanism is implemented. Preserve the
question-only invariance and narrow-exploration findings.
