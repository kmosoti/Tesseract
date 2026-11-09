# Workshop v1 protocol

Read the linked [research record](../knowledge/research/R-0001-substrate-evidence.md)
before interpreting the representation comparisons. This protocol qualifies a
small developmental instrument, not general intelligence.

## Frozen design

The executable configuration is [workshop-v1.json](../experiments/workshop-v1.json).
World-rule, sensor, and identity RNG streams are separately derived from named
SHA-256 seed material. The two public material contexts contain conjugated
four-state transition rules, including order-sensitive actions. These are
classical worlds; complex phases are not privileged by the generator.

Each seed supplies:

- 144 single-action training experiences and 24 heat-then-press training examples.
- 96 separate validation experiences over singles and pairs.
- 216 final evaluation experiences over previously unseen triples and fresh IDs.
- 32 goal-selection episodes, plus paired macro-enabled/disabled selections at
  one model evaluation per proposal. No evaluation feedback is trained on.

Training, validation, and evaluation sensor draws are separate, non-overlapping
segments of a named observation stream. Goal-selection sensors use an independent named evaluation stream,
paired across macro-enabled and disabled arms. All variants receive identical public
datasets. The kernel receives neither seeds nor world transition rules through
its experience interface. Its explicit fitting seed controls optimizer randomness.

Smoke seeds are 0–9; development seeds 100–119; final benchmark seeds 1000–1099.
Smoke/development verdicts remain diagnostic. Freeze implementation, protocol,
and relevant knowledge records before final benchmarking. Changes prompted by
benchmark results require a new protocol and untouched evaluation seeds.

## Controls and measurements

The primary metric is categorical Brier loss on held-out triples, lower is better.
Report every representation, including failed automatic promotions. Controls use
the same public evidence: nearest-prefix contextual retrieval, a Bayesian model
frozen after its first example per query, and observation storage without durable
model consolidation. The strongest control is selected by aggregate mean loss,
then held fixed across paired seed comparisons.

The primary hypothesis is **durable transition learning**. Learned operators must
transfer, survive actual file close/reopen and event replay, and remain unchanged
by intervening observations until validated consolidation. Conditional macros
have a separate causal test: disabling them changes the ordering of tightly
budgeted proposal search while holding the learned model fixed.

Report phase-removal and permutation-averaged prediction losses, goal success,
predictive entropy, action counts, objective evaluations, model/state/journal
bytes, fitting time, and peak RSS. Each seed/representation runs in an isolated
worker process. Linux workers and the coordinator each enforce a 1 GiB virtual
address-space ceiling; concurrency is restricted to 1–4 workers. This is a
per-process ceiling, not a 1 GiB aggregate limit across all workers.

The bootstrap uses 10,000 deterministic seed-level paired resamples and percentile
95% intervals for relative aggregate loss reduction. A developmental gate requires
at least 5% relative improvement over the strongest non-developmental control,
an interval strictly above zero, and all candidates committed with usable process
records. Zero-loss controls cannot support relative-improvement claims.

Complex-phase advancement compares against the strongest Bayesian/real candidate
and additionally requires fitting time and process peak RSS within 2× its comparator.
Report coordinate/byte capacity differences explicitly. Even a positive initial
gate requires capacity-matched replication and graph/rule plus VSA baselines
before any broader substrate claim. Overlapping seed subsets cannot be promoted
as independent replications.

## What remains unestablished

This first experiment does not test open-ended ontology growth, unrestricted
operator invention, lifelong streaming without finite evidence bounds, language
understanding, subjective experience, or deployment on the user's Fedora machine.
The simulator's full-state initial readings are more favorable than a general
partially observable environment. A model fitting a channel is not sufficient
evidence of discovering a new concept.
