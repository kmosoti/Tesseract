# Agent working agreement

Scope: this repository. Read README.md, CONTRIBUTING.md, docs/architecture.md,
and knowledge/README.md before changes. Inspect actual state; planned work is
not evidence that implementation exists. Preserve unrelated changes.

- Keep domain contracts independent of numeric kernels, storage, simulation,
  and evaluation. Kernels receive public experiences, never hidden world rules.
- Keep queries read-only. Separate working beliefs from validated durable
  competence. Every accepted change needs evidence, bounds, and a rollback path.
- Record decisions, research, observations, failures, and exact verification in
  the linked knowledge graph. Agent records are usable immediately with
  `added_by: agent` and `human_ratified: false`. Do not silently overturn an
  explicitly ratified human decision. A PR merge does not ratify records.
- Work on small branches or stacks based on `develop`. Prefer colocated `jj`;
  Git is supported. Delegate independent review and bottom-up landing into
  `develop` to a reviewer agent from another model family; if unavailable, use
  a fresh agent at the highest available reasoning effort. Do not self-merge.
- `main` releases require human review and the `main-release` environment gate.
  Agents never merge into `main` or approve that deployment gate. The initial
  minimal repository bootstrap is recorded separately from a release.
- Run relevant locked tests, formatting, Clippy, and knowledge validation.
  Report actual evidence and limitations. Negative results are valid outcomes.
- Never claim GitHub protections are active from workflow files alone.
