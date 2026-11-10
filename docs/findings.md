# Kernel v0.1: workshop verdict

The finite workshop supports persistent transition learning and the usefulness
of acquired macros under a tight search budget. It does **not** support advancing
the complex candidate over its strongest classical comparator.

The [raw evidence](../evidence/workshop-v1/receipt.json) covers seeds 1000–1099
across all three representations, using the frozen implementation at
`62cfd5b3b21eb5b28bfcc7389d3de3e83487ccc6`. Every one of the 300 candidates
committed automatically, retained 32 supported program records, survived actual
journal close/reopen, and passed read-only query and intervening-observation
checks. All 21 engineering tests and independent review also passed.

## Transfer and substrate comparison

| Representation | Mean held-out Brier loss | Reduction vs strongest non-developmental control | Paired 95% interval | Mean development/validation time |
| --- | ---: | ---: | ---: | ---: |
| Bayesian | 0.191976 | 59.18% | 57.86–60.45% | 0.496 ms |
| Real density | 0.175713 | 62.64% | 61.41–63.85% | 224.238 ms |
| Complex density | 0.178285 | 62.09% | 60.84–63.31% | 224.653 ms |

The strongest non-developmental control was the Bayesian model frozen after its
first example per query: mean loss 0.470313. Each developmental comparison clears
the preregistered 5% gate and paired interval above zero.

Against the real density candidate, the complex candidate's relative gain was
**−1.46%**, with paired 95% interval **[−2.08%, −0.86%]**. Its fitting-time ratio
was 1.002 and peak-RSS ratio 0.999, so the relative resource gate passed while
the predictive gate failed. This is a negative result for this bounded complex
candidate and protocol, not proof that all quantum-inspired methods are useless.

The Bayesian implementation remains the default. The matrix candidates had lower
loss here but required roughly 450 times its development/validation time and
about ten times its serialized model bytes. Their multi-step fitting objective
also uses training pairs that the simple tabular fit ignores. Their improvement
cannot be attributed solely to representation. A follow-up should match classical
multi-step learning, observation-noise treatment, capacity, and compute before
drawing a substrate conclusion.

## Acquired programs and diagnostics

With one model evaluation per proposal, acquired macros increased success from
**24.53% to 65.72%**, an absolute gain of 41.19 percentage points. Relative failure
reduction was 54.58%, with paired 95% interval [53.70%, 55.41%]. The macro-disabled
arm uses the same learned model and public goals. This establishes a causal
contribution of the bounded library in this search regime; it does not establish
open-ended program invention. All three representations share this macro learner
and therefore produce identical macro comparison results.

With the full bounded proposal search, all representations averaged 95% goal
success. Permutation-averaged prediction increased Brier loss to roughly 0.525–0.531
in all three, demonstrating the value of order information even classically.
The complex model's real-Choi diagnostic loss was 0.179603 versus its original
0.178285; this small within-model difference does not overturn the negative
complex-versus-real verdict.

## Environment, limits, and next experiment

Recorded machine: managed Linux x86_64, Intel Xeon Platinum 8573C, four available
logical CPUs, Rust 1.99.1; two isolated workers at a time. Mean worker peak RSS
was approximately 3.40 MiB Bayesian, 3.62 MiB real, and 3.62 MiB complex. Each
process enforced a 1 GiB address-space limit. Measurements include worker runtime
and controls and are observational, not device-independent performance promises.

The worlds have four states, known public initial readings, two public contexts,
and fixed bounded actions. Ontology growth, richer partial observability, graph
and VSA baselines, selective retraction, and execution on Fedora remain follow-up
work. No general-intelligence or consciousness claim is made.

See [publication status](publication.md) for the repository and release state.
Local evidence does not establish remote CI or branch protection.
