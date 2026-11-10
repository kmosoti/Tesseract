# Linked project knowledge

This versioned Markdown graph records evolving understanding. Stable operating
contracts live in `docs/`; runtime experiences live in journals. A research note
or implementation plan is not proof of competence.

Read related records before repeating research. Every non-index record has a
stable ID, type, status, update date, authorship, and explicit ratification state.
Agent records are usable immediately; a merge does not implicitly ratify them.
Only explicit human confirmation sets `human_ratified: true`, with `ratified_by`
and `ratified_on`. Do not silently overturn a ratified decision.

| Record | Type | Human ratified | Purpose |
| --- | --- | --- | --- |
| [D-0001](decisions/D-0001-developmental-kernel.md) | Decision | false | Developmental thesis and experimental substrates |
| [D-0002](decisions/D-0002-working-agreement.md) | Decision | false | Iterative development, review, and release model |
| [D-0003](decisions/D-0003-workshop-protocol.md) | Decision | false | Frozen comparison and advancement gates |
| [D-0004](decisions/D-0004-scientific-reading.md) | Decision | false | Declarative scientific reading and source provenance |
| [D-0005](decisions/D-0005-symbolic-interpretation.md) | Decision | false | Typed interpretation, translation boundaries, and bounded deductions |
| [R-0001](research/R-0001-substrate-evidence.md) | Research | false | Evidence, alternatives, and unanswered questions |
| [O-0001](observations/O-0001-bootstrap.md) | Observation | false | Starting environment and publication blocker |
| [O-0002](observations/O-0002-freeze-verification.md) | Observation | false | Independent qualification before frozen evaluation |
| [O-0003](observations/O-0003-workshop-verdict.md) | Observation | false | Supported transfer/macro learning; negative complex-substrate result |
| [O-0004](observations/O-0004-journal-recovery.md) | Observation | false | Journal recovery failure and publication status |
| [O-0005](observations/O-0005-symbolic-loop.md) | Observation | false | Complex symbolic feedback loop and language exploration |
| [O-0006](observations/O-0006-si-execution.md) | Observation | false | Executable SI, formal checks, and Rust tooling |
| [O-0007](observations/O-0007-english-adapter.md) | Observation | false | Bidirectional English adapter and bounded round trips |
| [O-0008](observations/O-0008-gpu-probe.md) | Observation | false | Optional Burn GPU batch and cache comparison |
| [O-0009](observations/O-0009-working-nucleus.md) | Observation | false | Working abstractions and disagreement-driven experiments |
| [O-0010](observations/O-0010-state-interview.md) | Observation | false | Read-only interview and evidence-driven state evolution |
| [O-0011](observations/O-0011-scientific-reading.md) | Observation | false | Wikipedia reading, general questions, and retrieval failures |

Each record distinguishes Observed, Inferred, Hypothesis, and Unknown; includes
typed relative links in Relations; and states Revisit when. Keep useful backlinks.
Supersede decisions explicitly rather than deleting their history. IDs are never
reused. Run `cargo xtask knowledge` to check graph integrity.

Use the following front matter for new records:

```yaml
---
id: O-0002
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
```
