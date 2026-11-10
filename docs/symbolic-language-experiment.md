# SI v0: grounded symbolic execution

SI is an executable, bounded language backed by Tesseract's persistent cognitive
state. Its first two operators are:

```text
predict porous 0 heat press
seek solid 1 3
```

`predict` applies an ordered program of one to three actions and returns a
probability distribution. `seek` selects a program for a target outcome under a
bounded search budget. States are 0..3; features are `porous` or `solid`; actions
are `heat`, `press`, and `cool`. Input is capped at 256 bytes. Parsing rejects
unknown operators, features, outcomes, actions, and invalid arities. Evaluation
also validates typed or deserialized terms through the kernel contracts.

The `tesseract-si` library returns a serializable `Thought` containing the term,
value, competence revision, and evidence head. Read-only evaluation does not
promote hypotheses. Complex-valued channels interpret action sequences when the
session uses `Kind::Complex`. Real-valued sessions use the same language.

Run against a saved journal:

```bash
cargo run --locked -p tesseract-si -- runs/demo.jsonl 'seek porous 0 2'
```

Thought records identify their source state; they are not yet a separately
journaled thought graph. Runtime evidence and accepted models use the existing
fsynced event journal. Natural-language prompts and external LLM calls are absent
from the language evaluator and intelligence test.

## Intelligence diagnostic

```bash
cargo test --locked -p tesseract-lab --test si_intelligence -- --nocapture
```

Three rounds each provide 24 fixed single-action exploration episodes and 32
SI-selected goal episodes. Each action produces a public observation that is
journaled. The learner develops from accumulated observations and uses the
unchanged validation policy before committing. Every round closes and reopens
the session and checks the full state digest. Fixed seed 101, zero sensor noise,
four fitting evaluations, and at most 39 proposal evaluations bound the run.

The matched controls use a complex kernel without consolidation and a real
kernel with consolidation. The diagnostic requires at least two accepted commits
and final loss below half the frozen control for both learning variants. It does
not require or establish a complex-over-real advantage. Candidate fitting starts
from accumulated empirical evidence, rather than warm-starting old channels.

Validation/evaluation identities are kept out of training. Evaluation action
programs can overlap goal feedback, and validation is reused; this is a diagnostic
of consolidation and persistence, not an untouched generalization benchmark.
Exhaustive exploration can explain the improvement independently of goal feedback.
Controller position and simulator RNG are not persisted across arbitrary crashes.

## Lean 4 and SMT

Install Lean via elan and Z3 on PATH. The repository pins Lean 4.24.0 and a dated
Rust 1.100 beta. `cargo xtask formal` checks the Lean model and SI solver tests.

Lean proves generic ordered action execution distributes over program composition,
a length bound for composition, and that predicted and observed evidence are
distinct constructors. These are abstract symbolic laws; there is no extracted
interpreter or theorem proving correspondence with Rust floating-point channels.

The Z3 check searches for a strictly better candidate among supplied probabilities,
encoded as exact IEEE-value rationals. Only `unsat` certifies maximality; errors,
unknown, timeout, and a counterexample all fail. Solver timeout is bounded. The
intelligence test checks selection against all 39 programs at two fixed goals per
round. This verifies ranking against current predictions, not their truth. A
negative solver test supplies a suboptimal selection and requires rejection.

Next language work requires explicit composition/dependency nodes, durable thought
records and controller state, independent validation streams, and an untouched
transfer protocol. The current slice establishes an executable public grammar and
a persistent learning cycle on a bounded world.

## English adapter

The Rust English adapter provides deterministic translation for the current SI
vocabulary. It accepts these sentence forms (case insensitive, with an optional
final period or question mark):

```text
Predict the outcome for a porous object starting in state 0 after heat then press.
What happens to a nonporous object in state 2 after cool then heat?
Find actions for a nonporous object starting in state 1 to reach state 3.
How can a porous object go from state 0 to state 2?
```

Features can be `porous`, `nonporous`, `non-porous`, or SI's `solid` alias.
English output uses `nonporous` for the latter three. Actions retain their order;
programs remain limited to three actions and state labels to 0..3. English input
is limited to 512 ASCII bytes. Unsupported wording, negation, unspecified state,
and out-of-domain concepts are rejected; the adapter supplies example templates.
It does not infer physical meanings for the numbered states.

```bash
cargo run --locked -p tesseract-si -- translate --to-si 'How can a porous object go from state 0 to state 2?'
cargo run --locked -p tesseract-si -- translate --to-english 'predict solid 1 heat cool'
cargo run --locked -p tesseract-si -- ask runs/demo.jsonl 'What happens to a porous object in state 0 after heat?'
cargo run --locked -p tesseract-si -- explain runs/demo.jsonl 'seek porous 0 2'
```

Translation requires no journal. `ask` evaluates the translated term against a
saved journal, then explains the result. `explain` evaluates SI directly and
renders the same English result. Explanations report all four probabilities for
predictions or the suggested program and estimated success for a goal, followed
by model revision and evidence head. Displayed percentages are rounded to two
decimal places. They remain estimates. Existing JSON CLI output is unchanged.

Every one of the 344 valid bounded terms is tested through canonical SI-to-English
and English-to-SI round trips. This ensures coverage of the present vocabulary;
it is not a claim of general natural-language understanding.
