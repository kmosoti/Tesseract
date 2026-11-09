# Contributing

## Repeatable development loop

1. Orient from actual code, branch state, linked knowledge, and open work.
2. Select a focused, testable change or stacked PR slice from `develop`.
3. Implement within the domain / kernel / storage / world / evaluator boundaries.
4. Run the checks for the changed behavior and record exact results.
5. Update linked decisions (`D-`), research (`R-`), observations (`O-`), and stable
   documentation where appropriate. Record unknowns rather than inventing proof.
6. Delegate review and rebase-merge into `develop` to an independent reviewer.
   Prefer another model family; use a fresh highest-effort reviewer otherwise.
7. Human reviewers promote `develop` to `main` through the `main-release` gate.

Agents do not wait for knowledge ratification. New agent-authored records have
`human_ratified: false`; only explicit human confirmation changes that field.

## Branches and commits

Prefer `jj git init --colocate` and bookmarks based on `develop@origin`.
Git branches and `git rebase --update-refs` are supported. Stacked PRs target their
immediate parent; the reviewer retargets and lands them bottom-up into `develop`.
Keep linear history and resolve all review threads. Use conventional commits:

```text
<type>(<scope>): <imperative summary>

Why: <problem>
What: <change>
Verification: <actual commands and limitations>
Knowledge: <linked record IDs>
```

## Checks

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/validate_knowledge.py
```

Use smoke seeds during implementation. Development seeds are separate from the
frozen evaluation seeds. Freeze the protocol before final comparisons; do not
tune against benchmark outcomes. Record failed and inconclusive verdicts.

## GitHub administration

Configure the `main-release` environment with human reviewer `kmosoti`, then
require `main-human-approval` and CI on `main`. Both branches require PRs, linear
history, and resolved conversations. Disable merge commits and squash merges.
`develop` permits independent agent integration without a GitHub human-review
count; independent review remains a repository working agreement. The
main branch also requires one eligible approving review; an author cannot approve
their own PR, so that review must come from another eligible human account.
Verify remote configuration using administrative reads. See docs/publication.md for
actual publication status; local files cannot enforce these settings.
