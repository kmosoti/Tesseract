---
id: O-0011
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Wikipedia scientific reading and general questions

## Observed

The reading corpus contains 15 revision-addressed English Wikipedia articles,
1,141 selected passages, 1,332 preserved source MathML expressions, and 4,795
normalized vocabulary terms. Its corpus digest is
`0c2ee70fb00694e2f963d855a1f5df55b4c5f5e2f582c9851a857e6c858df03b`.
All 15 archived revision HTML hashes and complete prose/MathML extraction matched.

The first frozen evaluation at source `c3609a1` exposed a false positive: an
unrelated electricity-bill question returned physics excerpts because its words
occurred in unrelated places in the corpus. It also failed to resolve an
instantaneous-communication paraphrase. Those negative results were retained.

Source `ec89cb28a498cda700853309bcbd132a636344e3` adds a per-passage coverage gate
and a general entanglement-inflection normalization. Every returned passage must
match at least half the question's content terms. A regression distinguishes
vocabulary scattered across unrelated passages from actual evidence co-occurrence.

The v2 protocol treats the original 18 questions as development/regression cases.
An independent observer authored five fresh held-out questions without executing
them; the source was committed before those questions ran. Reports recorded a
clean source tree. The development group produced 15 evidence bundles and three
abstentions; the fresh group produced four bundles and one abstention. These are
retrieval statuses, not scientific answer-accuracy scores.

The fresh questions retrieved directly relevant passages on Stern–Gerlach spin
quantization and quantum Darwinism. The double-slit which-path question retrieved
the disappearance of interference in its third excerpt; its first excerpt was
historical context. The Portugal-capital control abstained. The electricity-bill
regression now abstained, while the instantaneous-communication question still
returned related entanglement passages without resolving the communication claim.

All 23 questions were also issued through the SI CLI. Exact memory bytes remained
unchanged. Structured interview checks confirmed identical answers after reopen,
exact stored-passage references, and source-bound MathML references. No external
language model supplied runtime answers.

Local reproduction and verification commands were:

```bash
cargo run --locked -p tesseract-si -- read data/quantum-mechanics/corpus.json runs/wikipedia-reading/final-memory.json
cargo xtask wikipedia-verify data/quantum-mechanics/corpus.json runs/wikipedia-reading/source-html
cargo xtask reading-interview runs/wikipedia-reading/final-memory.json experiments/scientific-reading-v2.json runs/wikipedia-reading/v2-development.json
cargo xtask reading-interview runs/wikipedia-reading/final-memory.json experiments/scientific-reading-v2.json runs/wikipedia-reading/v2-held-out.json --held-out
cargo test --locked --workspace
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo xtask knowledge
cargo xtask formal
```

Report paths are exclusive. The local source archive was captured during import;
re-running source verification requires that archive. Reports, CLI transcripts,
raw HTML, and the earlier failed evaluation remain local operational artifacts.

## Inferred

Tesseract now has persistent, source-attributed scientific reading beyond the
workshop vocabulary. Changing the reading corpus changes returned content for the
same question; answers are not fixed question-answer templates. Useful retrieved
passages do not establish understanding or multi-step inference.

## Hypothesis

Explicit scientific propositions, premise handling, and semantic relations may
improve the unresolved paraphrase and context-selection failures. Any such claim
needs a new frozen evaluation rather than tuning against these held-out results.

## Unknown

The reader does not decide whether a premise is true, distinguish a historical
proposal from established proof, synthesize comparisons, or solve physics.
Wikipedia's scientific correctness and completeness were not independently
certified. The small diagnostic is not a general question-answering benchmark.

## Relations

- *implements*: [scientific reading boundary](../decisions/D-0004-scientific-reading.md)
- *documents*: [reading interface and limitations](../../docs/scientific-reading.md)
- *contrasts*: [workshop state interview](O-0010-state-interview.md)

## Revisit when

Semantic reasoning or trained representations replace lexical retrieval, or a
new frozen evaluation changes the observed abstention and paraphrase limitations.
