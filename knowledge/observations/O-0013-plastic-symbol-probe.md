---
id: O-0013
type: observation
status: preliminary
updated: 2026-11-12
added_by: agent
human_ratified: false
---
# Fixed-proposer plastic-symbol experiment

## Observed

The repository now contains a Rust-only integration benchmark comparing five
learners under a fixed synthetic three-program proposal distribution. No local
pretrained language model or external Metron runtime was available in the
execution environment used for initial prototyping. This benchmark cannot
validate frozen-small-LLM language transduction.

An independent JavaScript scratch simulator (not introduced into the Rust
repository) ran the prespecified final seeds 1000–1099 after smoke exploration
on seeds 0–9. It returned these preliminary mean Brier losses:

| Phase | Fixed | Retrieval | Family Bayes | Context Bayes | Plastic |
| --- | ---: | ---: | ---: | ---: | ---: |
| Warmup | 0.12410 | 0.12859 | 0.07472 | 0.04569 | 0.05597 |
| Transfer | 0.12836 | 0.09167 | 0.07580 | 0.03997 | 0.03997 |
| Novel | 0.06387 | 0.10161 | 0.06798 | 0.06882 | 0.06821 |
| Drift | 0.12643 | 0.09701 | 0.10839 | 0.04930 | 0.05123 |

The scratch-run transfer loss reduction against retrieval was 56.4%, with
seed-paired 95% bootstrap interval approximately [55.1%, 57.7%]. It was
approximately tied against the correctly partitioned context Bayes control.
The mechanism produced on average 2.18 family splits per seed; 18 of 100
seeds included a spurious split in the missing-candidate family.

These values are **reference-prototype measurements**, not yet independent
Rust compilation or GitHub CI evidence. If the Rust test differs, record
the discrepancy; do not silently edit the frozen parameters or final seeds.

## Inferred

The synthetic mechanism can select different executable hypotheses after
public feedback and transfer the contextual decision to fresh identities.
The matching performance of a conventional online Bayesian control means
this experiment does not establish a novel cognitive advantage.

Performance on an absent true program can be worse than the unchanged
proposer. An uncertainty distribution over a closed candidate set cannot
guarantee invention of an excluded operational meaning.

## Hypothesis

A source-separated frozen local language model may supply candidate ASTs
without repeated token generation during sensor-loop execution. Whether
this improves end-to-end capability, robustness, or wall-clock cost over
the same model with ordinary retrieval and tools remains untested.

## Unknown

- Whether the Rust implementation and GitHub CI reproduce the scratch results.
- Whether an actual frozen model produces the correct candidate program set.
- Whether a genuine concept-genesis operator can construct missing programs.
- Whether online updates remain robust under delayed/noisy/adversarial feedback.
- Whether this approach outperforms well-matched online Bayesian alternatives.

## Relations

- *tests*: [plastic-symbol protocol](../../docs/plastic-symbol-transfer-v0.md)
- *extends*: [working symbolic nucleus](O-0009-working-nucleus.md)
- *qualifies*: [developmental kernel thesis](../decisions/D-0001-developmental-kernel.md)

## Revisit when

The Rust CI checks are available, candidate programs are sampled from a
real frozen local model, or a candidate-generation operator is implemented.
