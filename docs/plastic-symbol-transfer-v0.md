# Plastic-symbol transfer v0: fixed-proposer mechanism probe

**Status:** exploratory synthetic benchmark, not a local-LLM experiment and not an SI/general-intelligence qualification.

## Question

Does online revision of competing executable meanings improve future prediction over
(a) an unchanged proposer and (b) ordinary finite episodic retrieval? Does automatic
context splitting provide any advantage not explained by standard context-aware
Bayesian learning?

The experiment deliberately does not run a pretrained language model or Metron.
The fixed proposer makes three Boolean programs available; all variants receive
the same candidates and the same stream of public observations. The simulator
contains a fourth latent program, withheld from the proposer. No algorithm is
given the generator's rule identity.

## Programs and data

- Candidate 0: `x2 > x1 && x1 > x0` (two strict increases).
- Candidate 1: `x2 >= 8 && x1 >= 8` (two elevated readings).
- Candidate 2: `x2 - x1 >= 4` (large last increase).
- Hidden program 3, **not available for selection**: `x2 >= 8 && x2 > x1 && x1 > x0`.
- Every reading is sampled independently from `0..12` using xorshift32; labels
  are flipped with 4% probability.
- Public context includes a family ID (0–3) and a mode bit (0–1). An unrelated
  object identity is never passed to the learners. The candidate meaning changes
  by family, by mode within one family, and through a late drift in another.
- Prespecified phases per seed: 360 warmup observations, 480 continuation
  observations from previously seen family/mode contexts (called 'transfer' in
  the output for compatibility), 240 out-of-candidate observations for family 3,
  and 480 drift observations. The continuation phase does NOT evaluate transfer
  to unseen domains or model explicit device identities.
- Final 100 seeds are 1000–1099. Smoke seeds 0–9 were inspected before the final
  comparison. Report all seeds. No final-seed tuning is permitted.

The synthetic proposer is a fixed prior `[0.60, 0.25, 0.15]` for the available
programs. It is not an LLM, model embedding, or demonstration of language
understanding. Calling it a frozen LLM would misrepresent the evidence.

## Prequential procedure

For every observation, each learner predicts `P(label=1)` **before** receiving
the label. Brier loss is `(prediction - label)^2`; lower is better.

1. **Fixed:** unchanged proposer mixture, no updates.
2. **Retrieval:** at most 100 recent public examples per family/mode, nine
   nearest raw sensor triples, distance-weighted and smoothed.
3. **Family Bayes:** online categorical hypothesis belief per family,
   deliberately ignorant of mode.
4. **Context Bayes (strong control):** the same belief update, directly
   indexed by known family *and* mode; has the correct partition in advance.
5. **Tesseract-style plastic:** pooled family-level hypothesis beliefs and
   parallel working mode-level beliefs. Once both modes have >=24 observations,
   disagree on their highest-probability interpretation, and have posterior
   maxima >=0.92, mint a mode-specific operational meaning for that family.

Beliefs use likelihood 0.96 for matching labels / 0.04 otherwise. Log evidence
is multiplied by 0.97 before incorporating each observation, a discounted
Bayesian heuristic rather than an exact stationary posterior. No gradient
updates or model training occur. The split rule, prior, memory bound and all
other parameters are common across final seeds.

## Measurement and falsification

Report per-phase mean Brier scores, a seed-paired 10,000-resample percentile
95% interval on relative transfer improvement against retrieval, numbers of
minted splits, and spurious splits in the unsupported family.

A positive relative gain against retrieval is evidence for this narrow,
candidate-covered, synthetic online-learning mechanism on familiar contexts. It is **not**
evidence for LLM replacement, superiority over well-specified online Bayesian
learning, independently invented programs, or open-ended concept genesis.

The prediction is that the context-aware Bayesian control will match or
outperform the adaptive system because its partition is supplied in advance.
In the novel phase, no available candidate represents the actual rule. This
is a deliberately adversarial coverage failure: only a genuine program
generator or substantially different approximation mechanism can recover
the missing hypothesis.

## Reproduce in Tesseract

```bash
cargo test --locked -p tesseract-lab --test plastic_symbols -- --nocapture
```

The Rust integration test prints one line prefixed
`TESSERACT_PLASTIC_SYMBOLS_RESULT=` with machine-readable JSON summary.
The test does not fail merely because a benchmark hypothesis fails.

## Next, genuinely decisive experiment

Capture candidate JSON ASTs from an **actual frozen 0.5B–4B model**, cache
them by prompt hash, verify no candidate set is altered across conditions,
and run that exact model in both arms. Use new natural-language instructions,
unseen device families and nontrivial delayed feedback; compare against
same-model retrieval, same-model contextual Bayesian updating, and an
ablation that cannot mint or split symbols. Freeze the evaluation protocol
before scoring. Add program synthesis when the true meaning is absent.

The current experiment is a prerequisite mechanism probe, not a substitute
for that larger evaluation.
