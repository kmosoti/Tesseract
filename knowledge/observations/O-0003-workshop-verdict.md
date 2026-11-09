---
id: O-0003
type: observation
status: completed
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Frozen workshop v1 result

## Observed

Ran the complete registered benchmark from clean source
`62cfd5b3b21eb5b28bfcc7389d3de3e83487ccc6`:

```text
cargo run --locked --release -p tesseract-lab -- compare --tier benchmark --workers 2 --out runs/workshop-v1-benchmark.json
```

All 300 seed/representation runs completed, committed, restored from disk, and
passed replay, read-only query, and intervening-observation checks. The strongest
non-developmental control had mean Brier loss 0.470313. Bayesian, real, and
complex candidates achieved 0.191976, 0.175713, and 0.178285 respectively.

The complex-versus-real gate failed: −1.46% relative gain, paired 95% interval
[−2.08%, −0.86%]. Learned macros increased one-evaluation search success from
24.53% to 65.72%; all representations use the same macro learner.

The [raw receipt](../../evidence/workshop-v1/receipt.json) and
[interpretation](../../docs/findings.md) preserve the source, protocol, machine,
executable hash, 10,000-resample intervals, and resource diagnostics. Fresh worker
reruns for all three representations on seed 1000 reproduced the saved behavioral
hashes; the complete benchmark was not rerun for tuning.

## Inferred

Retain the Bayesian default and the developmental architecture. Do not promote
complex channels on this evidence. The matrix candidates' improvement over the
tabular fit requires a better matched classical multi-step comparator; the
current difference includes objective/data-use and coordinate-cost differences.

## Hypothesis

A richer bounded developmental experiment may distinguish representation value
from program reuse and observation-noise modeling. This requires a new registered
protocol and untouched evaluation seeds.

## Unknown

Open-ended concept creation, lifelong scaling, selective retraction, VSA/graph
comparisons, Fedora performance, and public repository administration. The negative
complex result is limited to this instrument, optimizer, and world distribution.

## Relations

- *tests*: [D-0001](../decisions/D-0001-developmental-kernel.md)
- *observes*: [D-0003](../decisions/D-0003-workshop-protocol.md)
- *extends*: [O-0002](O-0002-freeze-verification.md)
- *challenges*: [R-0001](../research/R-0001-substrate-evidence.md)

## Revisit when

A new preregistered comparison supplies stronger counterevidence, reproducibility
fails, or richer environments change the developmental or substrate conclusion.
