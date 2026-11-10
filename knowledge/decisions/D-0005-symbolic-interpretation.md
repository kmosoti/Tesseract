---
id: D-0005
type: decision
status: accepted
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Symbolic interpretation and translation boundaries

## Observed

The original scientific SI Recall term preserved English and invoked lexical
retrieval. Translation did not infer an answer, but the term did not encode a
scientific proposition. Excerpts alone did not demonstrate symbolic interpretation.

The new reader extracts typed, revision-bound source claims independently of
questions. The compiler receives only question text. The engine receives a typed
query and an acquired graph. The renderer receives only the engine result.
Default scientific answers contain claims, proof steps and source witnesses;
paragraph retrieval remains an explicit inspection path.

## Inferred

Scientific interpretation should remain separate from linguistic conversion and
validated numerical competence. Signed claims, scope, attribution and hypothetical
premises must survive translation. A typed representation makes the engine's
actual operations inspectable without putting an answer in the translator.

## Hypothesis

Conservative source frames are a useful first test of rough symbolic understanding.
The supported frames include classification, loss-of, description, usability,
explicit aliases and an attributed linear-closure statement. The closure engine
rule is generic: it requires an acquired closure claim and two explicit solution
assumptions. It does not contain a fixed answer about a named physical theory.

## Unknown

The sentence parser is heuristic and incomplete. Producing an atom does not mean
that every clause of its source was understood. Noun-phrase labels retain shallow
meaning. This experiment neither validates Wikipedia nor demonstrates general
scientific understanding, new operator invention, or quantum advantage.

## Relations

- *extends*: [scientific reading boundary](D-0004-scientific-reading.md)
- *documents*: [source, query, engine and renderer](../../docs/scientific-reading.md)
- *contrasts*: [lexical reading observations](../observations/O-0011-scientific-reading.md)

## Revisit when

A new extraction or inference rule changes the interpretation contract. Persisted
graphs require an explicit extractor identity and a checked source derivation;
updates must not silently change what an existing memory represents.
