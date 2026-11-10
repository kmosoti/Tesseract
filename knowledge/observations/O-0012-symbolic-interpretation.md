---
id: O-0012
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Source acquired atoms and conditional deductions

## Observed

At source `6a9682cb1aee683fcd4f52d0179ef87ce876053c`, the unchanged Wikipedia
corpus produced 140 symbolic claims from 136 of 3,213 sentence segments; 3,077
segments produced none. A segment with an atom may still have unparsed clauses.
The extractor identity was `conservative-frames-1`; graph digest was
`285c9264b778ff9cd74becad168a59bd2156aeadbf2ec0e5901298e3dd9751ce`.

The stored model includes loss-of(coherence) for decoherence, an explicitly
negative faster-than-light usability claim for entanglement, and a separately
qualified negative transmit-information claim for entanglement alone. It also
contains a linear-closure assertion attributed to quantum superposition.
These are parsed source assertions, not independently validated physical laws.

The question compiler has no memory input. A CLI test translated a question before
any memory file existed. Altering source polarity changed the engine verdict
without changing that translation. The renderer receives only the resulting
structured proof and source references; it does not receive a corpus or index.

Six frozen-protocol questions ran through the symbolic engine and actual SI CLI.
The engine described decoherence and superposition, refuted the two scoped
usability queries, and derived a conditional linear-combination solution from
source closure and two explicit solution assumptions. The unsupported
consciousness/why question returned Unknown with no proof or admitted premise.
Memory bytes were unchanged and reopened answers were identical. Every proof
source and assumption reference was checked against its acquired graph or query.

Thirteen symbolic tests passed, including invented vocabulary, source
substitution/deletion, opposite-source conflicts, qualifier preservation,
question-versus-assertion distinction, subordinate negation, alias consistency,
conditional opposing assumptions, an indexed 600-pair disconnected alias workload,
cutoff diagnostics, and versioned source-bound persistence. A non-relation query
also ignored a separate 70-alias component rooted at the legitimate name `unused`.
The full workspace suite, formatting, Clippy, knowledge and formal checks passed.
The Lean addition checks only the generic closure inference given its premises.

Exact verification commands included:

```bash
cargo test --locked --workspace
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo xtask knowledge
cargo xtask formal
cargo run --locked -p tesseract-si -- read data/quantum-mechanics/corpus.json runs/symbolic-reading/final-memory.json
cargo run --locked -p tesseract-si -- inspect-symbols runs/symbolic-reading/final-memory.json
cargo xtask symbolic-interview runs/symbolic-reading/final-memory.json experiments/symbolic-understanding-v1.json runs/symbolic-reading/interview.json
```

The local run retains the graph, typed queries, compact proofs, CLI transcript,
immutable-memory checksums, and clean-source receipt. New output paths are needed
to repeat commands that publish memories or reports.

## Inferred

The model now exposes a narrow interpretation and an explicit deductive operation
beyond passage ranking. Source substitution and missing-premise tests distinguish
this behavior from a fixed answer in the translator. Classification labels,
relations and one acquired closure property are insufficient to claim broad
scientific comprehension.

## Hypothesis

Additional grounded operators and language coverage may expand the model, but
must preserve polarity, scope, attribution, conflicts and premise provenance.
Extending lookup with unsupported causal or classification transitivity would not
establish understanding.

## Unknown

Most source material remains uninterpreted. Phrase labels are shallow, arbitrary
English can be misparsed, and the source's scientific correctness is not proved.
The experiment does not normalize quantum states, solve equations, invent rules,
or demonstrate quantum advantage or general intelligence.

## Relations

- *implements*: [symbolic interpretation boundary](../decisions/D-0005-symbolic-interpretation.md)
- *extends*: [earlier lexical reading](O-0011-scientific-reading.md)
- *documents*: [interfaces and interpretation limits](../../docs/scientific-reading.md)

## Revisit when

New source frames, reasoning operators or memory versions change the interpretation
or deduction contract, or independent counterexamples reveal scope loss.
