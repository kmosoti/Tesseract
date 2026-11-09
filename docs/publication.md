# Publication status

**Local implementation; remote repository creation is blocked.**

On 2026-11-09, the authenticated GitHub integration returned
`Resource not accessible by integration (createRepository)` for creation of
public `kmosoti/tesseract`. No public repository, issues, remote CI, environments,
or branch protections have been created or verified by this session.

The local `main` branch contains the minimal bootstrap. Development and reviewed
implementation are kept on `develop` / its feature branch. Agents do not promote
this implementation to `main`; the release remains human-gated.

An owner-authorized setup is prepared in `scripts/publish.sh` and
`scripts/configure_github.py`. It must run with credentials permitting repository
creation and administration. The script does not print tokens, approve deployment
gates, or merge into `main`. It verifies settings before reporting success.

The issue seed file can be published idempotently after repository creation with
`python3 scripts/configure_github.py --issues`. If administration fails, keep the
specific failing action visible and do not claim protections are enforced.
