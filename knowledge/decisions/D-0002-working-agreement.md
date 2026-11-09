---
id: D-0002
type: decision
status: accepted
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Iterative development and asynchronous knowledge

## Observed

The user asked to follow their rust-ui-template development setup. Its
[contributor guide](https://github.com/kmosoti/rust-ui-template/blob/main/CONTRIBUTING.md)
and [knowledge protocol](https://github.com/kmosoti/rust-ui-template/blob/main/knowledge/README.md)
separate agent integration on develop from human-reviewed release on main.

## Inferred

Adopt focused slices, independent review, linear history, optional colocated jj,
exact verification records, and linked dated knowledge. Ratification must not
block ordinary agent work. Remote protections require administrative verification.

## Hypothesis

Independent review and linked counterevidence will reduce correlated engineering
mistakes and unsupported scientific interpretation while preserving iteration.

## Unknown

Remote repository administration is blocked. No main-release gate or branch
protection is currently enforced. No alternative model provider is available in
this session, so independent review uses a fresh highest-effort agent.

## Relations

- *implements*: [D-0001](D-0001-developmental-kernel.md)
- *observed-in*: [O-0001](../observations/O-0001-bootstrap.md)

## Revisit when

The branch model impedes measured iteration, an independent review route is
unavailable, or remote configuration differs from its documented contract.
