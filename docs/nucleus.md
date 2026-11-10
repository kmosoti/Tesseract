# Working concepts and curiosity

The nucleus is a bounded symbolic layer over public experience. It constructs
working abstractions, keeps counterexamples, and changes which experiment the
exploration loop selects. It works with Bayesian, real, or complex kernels.
Matrices remain one numerical predictor of action effects; symbolic concepts do
not require them. This separation allows the same concept and curiosity protocol
to compare numerical representations without making quantum-advantage claims.

## Concept language

A concept is a typed proposition:

```text
(porous condition | either condition, initial state | any initial state,
 ordered action program) -> predicted outcome
```

Omitting a condition is a generalization, not merely another object label. A
concept has a stable semantic hash, distinct supporting/counterexample object
counts, and up to eight observation references per side. References use the
journal's existing evidence identity scheme. Repeated observations of one object
do not count as several supporting objects. An omitted condition needs examples
across that condition before the claim is supported: both porosities or at least
two initial values, depending on the omission.

`candidate`, `supported_working`, and `contested` are working-belief statuses.
A single contradiction contests a deterministic universal claim but does not
reject a probabilistic grouping model. Three supporting objects, cross-condition
support where needed, and no counters qualify a supported working hypothesis.
This is not validation of an arbitrary universal law. Contradictions remain in
history. The UI shows up to sixteen concepts, prioritizing supported hypotheses;
truncation does not delete evidence.

## Attention and experiments

Four predefined structural hypotheses pool observations, split by porosity, split
by initial state, or split by both. Each has smoothed categorical predictions for
an ordered action program. Prequential Dirichlet-categorical evidence supplies
relative weights. Weighted Jensen-Shannon divergence measures disagreement.
The policy selects a query with greatest disagreement among the evaluated set;
score ties favor fewer observations. All-uniform cold start has no disagreement
and uses a coverage fallback. This is an explicit experiment-selection policy,
not a claim of an inner feeling or general curiosity.

There are 312 valid queries. Full-budget selection visits all of them. A smaller
budget uses an evidence-count-dependent permutation, so repeated selection cannot
permanently exclude the later contexts of a fixed lexicographic prefix. Bounds
are 4096 observations, 312 query evaluations, three actions, and at most 128
returned concepts through the API.

```bash
cargo run --locked -p tesseract-lab -- explore --journal runs/nucleus.jsonl --steps 48 --seed 114
cargo run --locked -p tesseract-si -- ask runs/nucleus.jsonl 'What concepts have you formed?'
cargo run --locked -p tesseract-si -- ask runs/nucleus.jsonl 'What should you investigate next?'
```

The corresponding SI terms are `concepts` and `wonder`. `explore` executes actual
workshop experiments, journals their observations, and attempts normal validated
numerical consolidation every sixteen steps. Rejections remain rejections. The
`--policy coverage` control uses the same step/fit budget and a fixed coverage
schedule. The final report includes decisions, public observations, evidence IDs,
promotion reports, concepts, next choice, loss, and replay checks.

Concepts and model weights are deterministic views reconstructed from existing
observation events. Querying them does not mutate the kernel. Reopening preserves
concepts, references, scores, and the next suggestion without adding a new journal
schema. Persisted working hypotheses remain separate from validated numerical
competence. A suggested next action is not itself a durable pending transaction;
arbitrary interrupted-controller/world resume is not implemented.

## Evidence and limits

Tests cover actual generalization across observed conditions, retained
counterexamples, entropy versus disagreement, posterior response to contrary
observations, small-budget coverage, malformed serialized reports, journal replay,
and a matched-policy diagnostic. The seed-114, noiseless, 48-observation
committee-prediction diagnostic gave Brier loss 0.6111872864 for coverage and
0.7368482906 for curiosity. Curiosity was worse. This tests the structural
committee prediction; the numerical kernel is a distinct predictor.

The fixed grouping family and predicate grammar are hand-authored. Their
supported instances arise from observations; the system does not invent arbitrary
predicates or new language primitives. Unknown action programs can yield uniform
agreement, and greedily pursuing disagreement can overfocus a few programs.
The policy remains an explicit optional experiment, not a demonstrated
improvement over coverage or a replacement for normal goal-directed selection.

The diagnostic reuses one world and holds out object identities, not programs or
world seeds. No superiority, unrestricted abstract reasoning, or consciousness
claim follows. Lean checks abstract working-status and action-composition laws;
it does not prove correspondence of this Rust implementation or floating-point
curiosity computation. Report validation checks structural consistency, not
cryptographic authentication of externally supplied evidence.
