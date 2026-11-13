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

GitHub Actions run [43358556163](https://github.com/kmosoti/Tesseract/actions/runs/43358556163)
on commit `49d9c545001f2ab422a59dacc827bdef870ed3cb` passed
`cargo fmt --all --check`, Clippy with denied warnings, the full locked
workspace tests, the dedicated plastic-symbol test, knowledge validation,
release build, existing diagnostic comparison, and Lean formal checks.
The compiled Rust benchmark reproduced these reference values to displayed
floating-point precision. Its machine-readable result was printed with the
`TESSERACT_PLASTIC_SYMBOLS_RESULT=` prefix in the Actions job log.

## Inferred

The synthetic mechanism can select different executable hypotheses after
public feedback and reuse contextual decisions on later independently sampled
traces from the same already-observed context families. This is in-domain
continuation, not demonstrated cross-domain or held-out-identity transfer.
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

- Whether an actual frozen model produces the correct candidate program set.
- Whether a genuine concept-genesis operator can construct missing programs.
- Whether online updates remain robust under delayed/noisy/adversarial feedback.
- Whether this approach outperforms well-matched online Bayesian alternatives.

## Relations

- *tests*: [plastic-symbol protocol](../../docs/plastic-symbol-transfer-v0.md)
- *extends*: [working symbolic nucleus](O-0009-working-nucleus.md)
- *qualifies*: [developmental kernel thesis](../decisions/D-0001-developmental-kernel.md)

## Revisit when

Candidate programs are sampled from a real frozen local model, a genuine
cross-domain transfer test is frozen, or a candidate-generation operator is
implemented.
