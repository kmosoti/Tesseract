#!/usr/bin/env python3
"""Owner-authorized setup; never approves a gate or merges a release."""
import argparse
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
REPO = "kmosoti/tesseract"


def api(path, method="GET", data=None):
    command = ["gh", "api", path, "--method", method]
    if data is not None:
        command += ["--input", "-"]
    result = subprocess.run(command, input=json.dumps(data) if data is not None else None,
                            text=True, capture_output=True, check=False)
    if result.returncode:
        raise SystemExit(f"GitHub setup failed at {method} {path}: {result.stderr.strip()}")
    return json.loads(result.stdout) if result.stdout.strip() else None


def configure():
    prefix = f"repos/{REPO}"
    api(prefix, "PATCH", {"allow_merge_commit": False, "allow_squash_merge": False,
                         "allow_rebase_merge": True, "delete_branch_on_merge": True})
    owner = api("users/kmosoti")
    api(f"{prefix}/environments/main-release", "PUT", {
        "reviewers": [{"type": "User", "id": owner["id"]}], "prevent_self_review": False})
    for branch in ["develop", "main"]:
        checks = ["checks"] + (["main-human-approval"] if branch == "main" else [])
        api(f"{prefix}/branches/{branch}/protection", "PUT", {
            "required_status_checks": {"strict": True, "contexts": checks},
            "enforce_admins": True,
            "required_pull_request_reviews": {"required_approving_review_count": 1 if branch == "main" else 0},
            "restrictions": None, "required_linear_history": True,
            "required_conversation_resolution": True, "allow_force_pushes": False,
            "allow_deletions": False})
    # Check actual responses, not just a successful write or local workflow.
    for branch in ["develop", "main"]:
        rules = api(f"{prefix}/branches/{branch}/protection")
        expected = {"checks"} | ({"main-human-approval"} if branch == "main" else set())
        assert expected <= set(rules["required_status_checks"]["contexts"])
        assert rules["required_linear_history"]["enabled"]
        assert rules["required_conversation_resolution"]["enabled"]
        assert rules["enforce_admins"]["enabled"]
        assert not rules["allow_force_pushes"]["enabled"]
        assert not rules["allow_deletions"]["enabled"]
        assert rules["required_pull_request_reviews"]["required_approving_review_count"] == (1 if branch == "main" else 0)
    environment = api(f"{prefix}/environments/main-release")
    reviewers = [r for rule in environment["protection_rules"]
                 if rule["type"] == "required_reviewers" for r in rule["reviewers"]]
    assert any(r["reviewer"]["id"] == owner["id"] for r in reviewers)
    repository = api(prefix)
    assert repository["visibility"] == "public"
    assert repository["allow_rebase_merge"] and not repository["allow_merge_commit"] and not repository["allow_squash_merge"]
    print("Verified remote branch protections and required human environment reviewer")


def issues():
    existing = subprocess.run(["gh", "issue", "list", "--repo", REPO, "--state", "all",
                               "--limit", "1000", "--json", "title"], text=True,
                              capture_output=True, check=True)
    titles = {item["title"] for item in json.loads(existing.stdout)}
    for issue in json.loads((ROOT / ".github/issue-seeds.json").read_text()):
        if issue["title"] not in titles:
            api(f"repos/{REPO}/issues", "POST", issue)
            titles.add(issue["title"])
    print("Issue seeds reconciled by exact title")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--issues", action="store_true", help="Seed work packages instead of configuring protections")
    args = parser.parse_args()
    issues() if args.issues else configure()
