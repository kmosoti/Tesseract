# Exploring a persistent symbolic language

The current experiment uses typed `Goal`, `Query`, `Experience`, and `Process`
values as grounded symbols. A goal selects a bounded action program using the
complex-valued model; the workshop returns a public experience; the journal
preserves that experience and accepted competence. No text prompt or LLM is
involved. This is a closed-loop diagnostic, not yet a new language interpreter.

A possible next language is a typed expression graph whose nodes distinguish:

- references to observed objects and public features;
- action programs and their composition, bounded by the existing action limit;
- predictions, labeled with the competence revision that produced them;
- observations, linked to journal evidence rather than asserted as facts;
- conditional goals and candidate processes, whose promotion still requires
  independent validation.

These node types would make the distinction between a hypothesis, an observation,
and an accepted process explicit. Complex states and channels would provide one
possible evaluator for predictions; a symbolic node would not imply physical
quantum computation. A sequential bytecode is another plausible representation:
it is simpler to replay but makes shared subexpressions and dependencies less
explicit. The graph's ownership, bounds, canonical form, validation, and replay
semantics must be designed and tested before either representation is adopted.

The `symbolic_loop` integration test seeds a complex kernel from a fixed public
training dataset, validates a revision, and runs two rounds of goal selection,
world interaction, journaled feedback, and attempted redevelopment. Rejected
candidates are not promoted. It checks read-only proposals, a difference from an
untrained complex model, evidence survival, and exact session replay. It reuses a
validation set and is a development diagnostic, not a held-out performance study.

The test does not persist a separate thought graph or controller position, prove
that newly observed feedback improves performance, or establish uninterrupted
crash recovery for the simulated world. These are open work. A stronger evaluation
needs frozen-complex, real-valued, and memory-reset controls, independent
validation and evaluation streams, and explicit action and compute budgets.
