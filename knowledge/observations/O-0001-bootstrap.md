---
id: O-0001
type: observation
status: historical-snapshot
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Starting state and public-repository blocker

## Observed

On 2026-11-09, /workspace contained no Tesseract checkout. The connected GitHub
repository listing and named repository search did not locate a Tesseract source.
The user authorized creating a public repository if necessary.

Created a local minimal bootstrap commit and main/develop branches. GitHub rejected
`gh repo create kmosoti/tesseract --public` with
`Resource not accessible by integration (createRepository)`. No remote publication
or protections can be claimed. No credentials were copied into this repository.

The environment initially lacked Rust. Installed the official stable toolchain,
observed Rust 1.99.1, and pinned that exact version with rustfmt and Clippy.
Git 2.52.0 and gh were available; jj was absent, so the documented Git fallback is
used. This managed Linux environment is not the user's Fedora hardware.

## Inferred

Complete and verify the local implementation without pausing for administrative
access. Prepare reproducible publication and configuration scripts, an issue seed
file, and a reviewable local history.

## Hypothesis

An owner-authorized credential will permit publication without changing the
reviewed implementation or relaxing human release ownership.

## Unknown

Whether prior Tesseract source exists elsewhere; whether earlier numerical claims
are reproducible; whether repository-administration permissions will be supplied.

## Relations

- *observes*: [D-0001](../decisions/D-0001-developmental-kernel.md)
- *constrains*: [D-0002](../decisions/D-0002-working-agreement.md)

## Revisit when

Public creation/administration succeeds or historical source becomes available.
