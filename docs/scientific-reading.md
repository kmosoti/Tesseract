# Scientific reading memory

Tesseract can read a local Wikipedia corpus and retrieve source-attributed
passages for ordinary English questions. The supplied corpus covers 15 quantum
mechanics topics, including superposition, measurement, uncertainty, entanglement,
tunnelling, decoherence, and interpretations. It runs offline after import.

## Read and ask

From the repository root, create a new reading memory:

```bash
mkdir -p runs
cargo run --locked -p tesseract-si -- read data/quantum-mechanics/corpus.json runs/quantum-reading.json
cargo run --locked -p tesseract-si -- ask-knowledge runs/quantum-reading.json 'Can quantum entanglement send messages faster than light?'
cargo run --locked -p tesseract-si -- ask-knowledge runs/quantum-reading.json 'What is the Born rule?'
```

Existing memories are never overwritten. Reading records the corpus with a
content digest; reopening validates it and reconstructs the index. Questions are
read-only. They do not modify the corpus or install workshop transition models.
No network, GPU, downloaded language model, or external answer service is used
when asking questions.

The scientific SI term is `recall` followed by a JSON string. It preserves the
question, including Unicode and escaped quotation marks:

```bash
cargo run --locked -p tesseract-si -- translate --to-english 'recall "What is quantum superposition?"'
cargo run --locked -p tesseract-si -- recall runs/quantum-reading.json 'recall "What is quantum superposition?"'
```

The latter returns a structured reading thought with the question, corpus digest,
source passages, revision URLs, passage digests, and any referenced source MathML.
Workshop SI commands and journals keep their existing interfaces.

## Evidence and limits

The reader normalizes words, applies a small synonym vocabulary, and ranks
passages using lexical relevance and article-title priors. It returns at most
three excerpts and reports insufficient evidence when too few question terms
occur in the corpus. Relevance scores are neither probabilities nor scientific
confidence. Unsupported premises can still retrieve related text: asking whether
consciousness proves collapse does not make that claim established.

Answers are excerpts, not generated explanations or multi-step deductions.
Comparisons can retrieve both subjects without composing a comparison. Negation,
causal explanations, paraphrases, and questions about scientific proof require
careful inspection of the returned evidence. An out-of-scope question can share
words with these articles and pass the lexical gate. This mechanism does not
certify entailment, resolve competing interpretations, or calculate physics.

Equations remain source presentation MathML. Prose contains explicit equation
references; the CLI links to the typeset source. The reader does not turn visual
scripts into algebra or evaluate expressions. Lean and SMT checks for workshop
SI do not establish the truth of imported Wikipedia claims.

## Sources and updating

The [dataset notice](../data/quantum-mechanics/NOTICE) describes attribution,
modifications, and the separate CC BY-SA 4.0 license. Each article retains revision
and retrieval timestamps and a SHA-256 of the retrieved HTML. Source revisions
are requested using MediaWiki `oldid`. Rendered templates can depend on other
revisions; the stored corpus digest identifies the exact reading snapshot.
Unkeyed digests detect corruption, not a maliciously rewritten corpus.

An explicit update requires curl and network access. Use a new output and archive
directory, then review the source changes before using the new corpus:

```bash
cargo xtask wikipedia data/quantum-mechanics/topics.json runs/new-corpus.json runs/new-source-html
cargo xtask wikipedia-verify runs/new-corpus.json runs/new-source-html
```

The importer selects prose and native MathML, not every table, illustration, or
list in the articles. The optional HTML archive enables offline verification of
source hashes and extraction. Article and memory counts, question sizes, input
files, and response sizes have explicit bounds; oversized inputs fail.

## Reproduce an interview

The protocol separates development questions from held-out paraphrases. After
building a memory, choose new report paths:

```bash
cargo xtask reading-interview runs/quantum-reading.json experiments/scientific-reading-v1.json runs/reading-development.json
cargo xtask reading-interview runs/quantum-reading.json experiments/scientific-reading-v1.json runs/reading-held-out.json --held-out
```

Each command writes JSON evidence and a text transcript. It checks exact passage
and equation references, unchanged memory bytes, and identical answers after
reopen. These checks establish persistence and provenance, not answer accuracy;
scientific relevance needs separate review.
