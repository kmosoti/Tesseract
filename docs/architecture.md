# Architecture and verification scope

Tesseract v4's architectural thesis is persistent, revisable competence grounded
in interaction. Kernel v0.1 tests a bounded instance of that thesis. It acquires
transition operators and conditional sequence macros; it does not invent arbitrary
primitives, a general ontology, or an autonomous scientific method.

## Boundaries

| Component | Owns | Cannot depend on |
| --- | --- | --- |
| contracts | Public messages, budgets, identity references, probability contracts | Numerical libraries, storage, worlds |
| kernel | Working symbolic records, models, candidate fitting, macros, validation, revisions | Workshop, filesystem, CLI, hidden rules |
| journal | Single-writer event storage, replay, checkpoints | Simulator truth |
| workshop | Private seeded material rules, noisy final sensor readings | Cognitive implementation |
| lab | Protocols, controls, paired evaluation, resource measurements, CLI | Privileged access passed into kernels |
| knowledge | Source-attributed reading snapshots, passage retrieval, source mathematics | Numerical kernels, simulator truth, external answer services |

The workshop begins an episode with a public four-category sensor state and a
public porous/nonporous feature. Heat, press, and cool act on private material
rules. An experience exposes the initial reading, the chosen action sequence,
and its final noisy reading. Intermediate hidden states are not supplied.

## Cognitive contract

- `observe` validates and appends public evidence, preserves grounded object IDs,
  and updates the latest working interpretation. It does not change predictions.
- `predict` is read-only and returns four probabilities, entropy, and revision.
- `propose` searches programs of one to three actions within an evaluation budget.
  Acquired macros with matching public conditions and target outcomes are tried
  first. `information` is predictive entropy, **not** expected information gain.
- `develop` fits a candidate and mines supported conditional sequence macros.
  A macro needs at least three distinct training objects and evidence links.
- `validate` compares against the actual parent on disjoint public validation
  objects, checks retained earlier validation episodes, numerical validity,
  fit-report consistency, and recomputed macro provenance. Its sealed proof binds
  the model kind, parent content, revision, and observation head.
- `commit` installs only a current sealed proof. Default automatic promotion
  requires 1% relative validation loss reduction and at most 0.01 absolute retained
  Brier-loss regression. These are operational checks; the experiment's separate
  advancement gate requires 5% held-out relative improvement with uncertainty.
- `rollback` selects a prior complete competence revision and preserves the
  observation and commit history. It preserves competence in that revision;
  selective removal of an old process while retaining later dependent revisions
  is not implemented.

Accepted validation identities cannot later enter training, including after
rollback. Sessions allow at most 4096 observations and 1024 retained validation
episodes; budget exhaustion is explicit rather than silent forgetting.

## Representations

The Bayesian baseline learns categorical transition rows with symmetric 0.25
pseudocounts, indexed by public feature, action, and initial state. Composition is
ordinary Markov prediction. Only single-action examples fit these rows.

The real and complex density candidates use dimension four, sixteen Kraus blocks
per channel, and six context/action channels. Stacked isometries are constructed
using twice-reorthogonalized thin QR. CPTP channels preserve valid local states;
ordinary queries never apply measurement updates. A separate projective
conditioning operation exposes its likelihood and rejects impossible outcomes.

Density candidates begin at an exact classical transition embedding and optimize
training Brier loss over single- and multi-action examples by seeded coordinate
search. The frozen protocol permits 192 objective evaluations. This is a hybrid
empirical initialization, not a demonstrated quantum learning-from-scratch method.

`parameter_scalars` reports 72 free row coordinates for the Bayesian model versus
1536/3072 stored real/complex Kraus coordinates. These are different conventions,
not equal intrinsic capacities. Serialized bytes are reported separately. An
advancement claim requires additional capacity-matched replication.

The phase diagnostic averages a channel with its coefficient conjugate in the
fixed public basis, equivalently taking the real Choi matrix and extracting real
Kraus operators. It is independent of Kraus gauge, but still basis dependent.
The order diagnostic averages predictions over action permutations. Neither is a
physical quantum measurement or a proof that order effects are uniquely quantum.

## Storage and reproducibility

A session clones a prospective state, appends and fsyncs its event, then exposes
the new state. A file lock excludes concurrent writers. JSONL records carry schema,
sequence, previous digest, and payload digest. Replay revalidates commits against
their stored public validation episodes. Incomplete final lines can be recovered;
complete corrupt records fail. Journals are limited to 64 MiB.

If an append fails and restoring and syncing the original length also fails, the
session rejects further writes and checkpoint operations. Close and reopen it to
replay the journal; an append that returned an error may nevertheless be present
if its complete record reached disk before recovery failed.

Checkpoints are atomically published, exclusive snapshots checked against journal
replay. They do not replace the verified event stream. Unkeyed hashes detect
corruption and bind evidence; they do not authenticate an attacker who can rewrite
the entire chain. A trusted externally retained digest supplies that anchor.

Behavioral hashes exclude clocks, RSS, worker scheduling, and temporary paths.
Numeric reproducibility is asserted for the pinned environment and deterministic
ordering, not bitwise equivalence across all BLAS implementations or platforms.

## Experimental symbolic nucleus

The [working nucleus](nucleus.md) consumes public observation events and derives
conditional propositions and structural hypotheses. It does not read hidden
workshop rules, replace numerical predictions, or install validated competence.
The lab exploration controller can use its disagreement policy to choose actual
experiments; `concepts` and `wonder` expose read-only views through SI. Existing
journal events reconstruct the view, avoiding a persisted-schema change. This is
bounded abstraction over known predicates, not arbitrary ontology invention.

## Declarative scientific reading

The [scientific reader](scientific-reading.md) accepts a separately imported
corpus, preserves revision provenance and source MathML, and builds a deterministic
lexical index. SI `recall` terms evaluate against this reading memory, returning
excerpts and their evidence references. Read-only questioning cannot install
validated competence or alter workshop predictions. This boundary prevents
imported prose from becoming a sealed numerical learning result. Corpus reading
is persistent knowledge acquisition; retrieval alone does not establish semantic
entailment, truth, or general scientific reasoning.
