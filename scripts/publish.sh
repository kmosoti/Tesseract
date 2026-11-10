#!/usr/bin/env bash
# Run with owner-authorized GitHub credentials after reviewing the local history.
set -euo pipefail
cd "$(dirname "$0")/.."
if ! gh repo view kmosoti/tesseract --json nameWithOwner >/dev/null 2>&1; then
  gh repo create kmosoti/tesseract --public --description \
    'A Rust laboratory for persistent developmental learning and controlled classical versus quantum-inspired experiments'
fi
if ! git remote get-url origin >/dev/null 2>&1; then
  git remote add origin https://github.com/kmosoti/tesseract.git
fi
test "$(git remote get-url origin)" = 'https://github.com/kmosoti/tesseract.git'
# The initial main bootstrap is not a release promotion. Never merge main here.
git push origin main develop
gh repo edit kmosoti/tesseract --default-branch main
cargo xtask configure-github
cargo xtask issues
