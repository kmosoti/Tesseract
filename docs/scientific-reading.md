# Scientific reading and symbolic interpretation

Tesseract reads a local Wikipedia corpus into source-attributed symbolic claims.
The default scientific answer shows those claims and bounded proof steps. The
English adapter parses questions and renders results; the symbolic engine owns
reasoning. This is a rough interpretation of a small set of sentence forms, not
broad understanding of scientific prose.

## Read and inspect

Create a new memory; existing files are never overwritten:

```bash
mkdir -p runs
cargo run --locked -p tesseract-si -- read data/quantum-mechanics/corpus.json runs/quantum-symbolic.json
cargo run --locked -p tesseract-si -- inspect-symbols runs/quantum-symbolic.json
cargo run --locked -p tesseract-si -- ask-knowledge runs/quantum-symbolic.json 'What is quantum superposition?'
cargo run --locked -p tesseract-si -- ask-knowledge runs/quantum-symbolic.json 'Can entanglement be used for faster-than-light communication?'
```

The inspector exposes the acquired graph, extraction rules, source byte spans,
revision URLs, corpus digest, graph digest, and extraction counts. A sentence
counted as having atoms may still contain unparsed clauses. Phrase labels remain
shallow concepts; a label is not a complete interpretation of its meaning.

Example internal claims acquired from the supplied corpus, in abbreviated form:

```text
loss_of(quantum_decoherence, quantum_coherence)
not usable_for(entanglement, faster_than_light_communication)
not usable_to(entanglement alone, transmit_information)
according_to(quantum_superposition,
  closed_under(solutions(schrodinger_equation), linear_combination))
```

These are working source interpretations, not validated physical laws. Every atom
has a revision-bound witness. Negation, `alone`, and attribution are part of the
representation. An unqualified query cannot silently use an `alone` claim.

## Translation and reasoning boundaries

The pipeline has four explicit parts:

1. The reader extracts typed claims from stored source sentences, independently
   of questions. Unsupported forms are left unparsed.
2. The question compiler converts a supported English form into a typed query.
   It has no memory input and returns no verdict or scientific answer.
3. The engine compares claims, resolves uncontested aliases, and applies its
   explicitly implemented inference rules.
4. The renderer receives only the structured engine result, including verdict,
   proof steps, assumptions and source references. It cannot search the corpus.

Inspect translation without loading any memory:

```bash
cargo run --locked -p tesseract-si -- translate --science-to-si 'Can entanglement be used for faster-than-light communication?'
```

SI uses `reason` followed by the typed query as JSON. `reason MEMORY SI` returns
the structured result; `translate --to-english SI` describes a query without
evaluating it. Workshop `predict`, `seek`, `concepts` and `wonder` keep their
existing interfaces and numerical state contracts.

The source reader supports bounded classification, description, loss-of,
can-produce, signed use-for/use-to, explicit aliases, and one linear-closure frame.
It rejects tested interrogative, conditional, unresolved-pronoun and qualified
forms instead of dropping their qualifiers. This heuristic parser has no claim
of soundness or completeness for arbitrary English. Ambiguous qualifiers and
many valid scientific sentences remain uninterpreted.

## A conditional deduction

```bash
cargo run --locked -p tesseract-si -- ask-knowledge runs/quantum-symbolic.json 'Under quantum superposition, if a and b are solutions of the Schrödinger equation, is their linear combination also a solution?'
```

The compiler records the two assumptions and the requested conclusion. The
engine needs the separately acquired closure claim to derive membership of the
linear combination. Deleting that claim produces Unknown. The result stays
conditional: it does not assert that specific observed objects are solutions,
that the combination is normalized, or that it is an observed physical state.

The Lean theorem checks this generic closure inference schema given its premises.
It does not verify Wikipedia, the English extractor, or the Rust implementation.

## Verdicts and bounds

Verification distinguishes Supported, Refuted, Conflict and Unknown relative to
the parsed claims. Missing evidence is Unknown. Opposite assertions remain
visible; they do not authorize arbitrary conclusions. The engine does not apply
generic causal, descriptive or classification transitivity. It withholds a
conditional deduction when an explicit assumption opposes the conclusion or a
required source premise.

Aliases are indexed once per query. A negative alias inside a positive
equivalence component disables substitution for that component, with source
witnesses. Normal traversal is limited to four edges and 64 concepts; cutoff and
budget failures are explicit. Displays retain conflicting witnesses before the
16-entry proof limit. Source interpretation is limited to 32,768 sentence
segments, 4,096 atoms and 4 MiB of graph data. Combined memories are at most
16 MiB. These are operational bounds, not accuracy guarantees.

## Persistence and earlier memories

Schema 2 stores the corpus and its exact interpretation with extractor identity
and digests. Reopen checks the stored graph against the same versioned derivation;
editing a graph and recomputing a hash cannot substitute a different derivation.
An incompatible extractor requires explicit reinterpretation. Questions are
read-only and do not install workshop competence.

Schema 1 memories remain available for their existing retrieval behavior. They
are not silently reinterpreted. To acquire symbolic claims, read the original
corpus into a new schema 2 memory with `read`.

Full paragraph retrieval is an explicit inspection option:

```bash
cargo run --locked -p tesseract-si -- retrieve-knowledge runs/quantum-symbolic.json 'What is the Born rule?'
cargo run --locked -p tesseract-si -- recall runs/quantum-symbolic.json 'recall "What is the Born rule?"'
```

The old retrieval index still has lexical relevance and premise-handling limits.
Its excerpts are not the engine's symbolic understanding and are not used as a
fallback when a symbolic question is Unknown.

## Sources and reproducible runs

The [dataset notice](../data/quantum-mechanics/NOTICE) describes contributor
attribution, modifications, and the separate CC BY-SA 4.0 license. The 15 articles
retain revision/retrieval timestamps and source HTML hashes. Native MathML is
preserved without guessing algebra from visual scripts. The current symbolic
reader does not interpret these mathematical expressions; its closure frame is
acquired from an explicit prose statement.

An explicit source update uses curl and network access; asking questions needs
neither a network service, GPU nor external language model:

```bash
cargo xtask wikipedia data/quantum-mechanics/topics.json runs/new-corpus.json runs/new-source-html
cargo xtask wikipedia-verify runs/new-corpus.json runs/new-source-html
cargo xtask symbolic-interview runs/quantum-symbolic.json experiments/symbolic-understanding-v1.json runs/symbolic-interview.json
```

Rendered templates can depend on other revisions; corpus digests identify the
exact retained snapshot. Unkeyed hashes detect corruption, not a maliciously
rewritten corpus. The interview writes typed queries, compact results, proof
references and a text transcript, and checks immutable memory and exact reopen
results. The earlier lexical protocols remain available through
`cargo xtask reading-interview` for reproducing retrieval diagnostics.
