use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
fn require(ok: bool, message: &str) -> Result {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn run(command: &mut Command) -> Result<String> {
    let output = command.output()?;
    require(
        output.status.success(),
        &format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )?;
    Ok(String::from_utf8(output.stdout)?)
}
fn api(path: &str, method: &str, body: Option<Value>) -> Result<Value> {
    let mut command = Command::new("gh");
    command.args(["api", path, "--method", method]);
    if body.is_some() {
        command.args(["--input", "-"]);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(body) = body {
        child
            .stdin
            .take()
            .ok_or("missing stdin")?
            .write_all(body.to_string().as_bytes())?;
    }
    let output = child.wait_with_output()?;
    require(
        output.status.success(),
        &format!(
            "{method} {path}: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    )?;
    Ok(if output.stdout.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&output.stdout)?
    })
}
fn configure() -> Result {
    let prefix = "repos/kmosoti/Tesseract";
    api(
        prefix,
        "PATCH",
        Some(
            json!({"allow_merge_commit":false,"allow_squash_merge":false,"allow_rebase_merge":true,"delete_branch_on_merge":true}),
        ),
    )?;
    let owner = api("users/kmosoti", "GET", None)?;
    api(
        &format!("{prefix}/environments/main-release"),
        "PUT",
        Some(json!({"reviewers":[{"type":"User","id":owner["id"]}],"prevent_self_review":false})),
    )?;
    for branch in ["develop", "main"] {
        let count = usize::from(branch == "main");
        let checks = if count == 1 {
            vec!["checks", "main-human-approval"]
        } else {
            vec!["checks"]
        };
        let path = format!("{prefix}/branches/{branch}/protection");
        api(
            &path,
            "PUT",
            Some(
                json!({"required_status_checks":{"strict":true,"contexts":checks},"enforce_admins":true,"required_pull_request_reviews":{"required_approving_review_count":count},"restrictions":null,"required_linear_history":true,"required_conversation_resolution":true,"allow_force_pushes":false,"allow_deletions":false}),
            ),
        )?;
        let rules = api(&path, "GET", None)?;
        for check in checks {
            require(
                rules["required_status_checks"]["contexts"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!(check))),
                "missing required check",
            )?;
        }
        for flag in [
            "required_linear_history",
            "required_conversation_resolution",
            "enforce_admins",
        ] {
            require(rules[flag]["enabled"] == true, flag)?;
        }
        for flag in ["allow_force_pushes", "allow_deletions"] {
            require(rules[flag]["enabled"] == false, flag)?;
        }
        require(
            rules["required_pull_request_reviews"]["required_approving_review_count"] == count,
            "review count mismatch",
        )?;
    }
    let environment = api(&format!("{prefix}/environments/main-release"), "GET", None)?;
    require(
        environment["protection_rules"]
            .as_array()
            .is_some_and(|rules| {
                rules.iter().any(|r| {
                    r["type"] == "required_reviewers"
                        && r["reviewers"]
                            .as_array()
                            .is_some_and(|rs| rs.iter().any(|r| r["reviewer"]["id"] == owner["id"]))
                })
            }),
        "missing human environment reviewer",
    )?;
    let repo = api(prefix, "GET", None)?;
    require(
        repo["visibility"] == "public"
            && repo["allow_rebase_merge"] == true
            && repo["allow_merge_commit"] == false
            && repo["allow_squash_merge"] == false,
        "repository settings mismatch",
    )?;
    println!("Verified remote governance");
    Ok(())
}
fn issues() -> Result {
    let existing: Value = serde_json::from_str(&run(Command::new("gh").args([
        "issue",
        "list",
        "--repo",
        "kmosoti/Tesseract",
        "--state",
        "all",
        "--limit",
        "1000",
        "--json",
        "title",
    ]))?)?;
    let mut titles: BTreeSet<String> = existing
        .as_array()
        .ok_or("invalid issues")?
        .iter()
        .filter_map(|v| v["title"].as_str().map(str::to_owned))
        .collect();
    let seeds: Vec<Value> = serde_json::from_slice(&fs::read(".github/issue-seeds.json")?)?;
    for seed in seeds {
        let title = seed["title"].as_str().ok_or("missing title")?.to_owned();
        if titles.insert(title) {
            api("repos/kmosoti/Tesseract/issues", "POST", Some(seed))?;
        }
    }
    Ok(())
}
fn valid_date(date: &str) -> bool {
    let parts: Vec<_> = date
        .split('-')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    if parts.len() != 3
        || date.len() != 10
        || !date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    {
        return false;
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    let leap = y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let days = match m {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    y > 0 && d > 0 && d <= days
}
fn knowledge() -> Result {
    knowledge_at(Path::new("."))
}
fn knowledge_at(root: &Path) -> Result {
    let index = fs::read_to_string(root.join("knowledge/README.md"))?;
    let mut ids = BTreeSet::new();
    for (folder, prefix, kind) in [
        ("decisions", "D", "decision"),
        ("research", "R", "research"),
        ("observations", "O", "observation"),
    ] {
        for entry in fs::read_dir(root.join(format!("knowledge/{folder}")))? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            let mut parts = text.splitn(3, "---");
            require(
                parts.next().is_some_and(|s| s.trim().is_empty()),
                "missing front matter",
            )?;
            let fields: BTreeMap<_, _> = parts
                .next()
                .ok_or("missing fields")?
                .lines()
                .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim(), v.trim())))
                .collect();
            for key in [
                "id",
                "type",
                "status",
                "updated",
                "added_by",
                "human_ratified",
            ] {
                require(fields.contains_key(key), key)?;
            }
            let id = fields["id"];
            require(
                id.len() == 6
                    && id.starts_with(&format!("{prefix}-"))
                    && id[2..].bytes().all(|c| c.is_ascii_digit())
                    && ids.insert(id.to_owned()),
                "invalid/duplicate id",
            )?;
            require(
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(&format!("{id}-")),
                "filename mismatch",
            )?;
            require(
                fields["type"] == kind
                    && ["agent", "human", "mixed"].contains(&fields["added_by"])
                    && ["true", "false"].contains(&fields["human_ratified"]),
                "invalid provenance",
            )?;
            require(valid_date(fields["updated"]), "invalid date")?;
            if fields["human_ratified"] == "true" {
                require(
                    fields.get("ratified_by").is_some_and(|v| !v.is_empty())
                        && fields.get("ratified_on").is_some_and(|d| valid_date(d)),
                    "missing ratification",
                )?;
            }
            require(index.contains(id), "unindexed record")?;
            for section in [
                "Observed",
                "Inferred",
                "Hypothesis",
                "Unknown",
                "Relations",
                "Revisit when",
            ] {
                require(text.contains(&format!("## {section}\n")), section)?;
            }
            require(
                text.lines()
                    .any(|l| l.starts_with("- *") && l.contains("*: [")),
                "missing typed relation",
            )?;
            for rest in text.split("](").skip(1) {
                let target = rest.split(')').next().ok_or("bad link")?;
                if !target.contains("://") && !target.starts_with('#') {
                    require(
                        path.parent()
                            .unwrap()
                            .join(target.split('#').next().unwrap())
                            .exists(),
                        &format!("broken link: {target}"),
                    )?;
                }
            }
        }
    }
    println!("Validated {} linked records", ids.len());
    Ok(())
}
fn reproduce(args: &[String]) -> Result {
    let root = std::env::current_dir()?;
    let receipt_path = root.join(args.first().ok_or("receipt required")?);
    let binary = root.join(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("target/release/tesseract"),
    );
    if let Some(source) = args.get(2) {
        std::env::set_current_dir(source)?;
    }
    let receipt: Value = serde_json::from_slice(&fs::read(receipt_path)?)?;
    require(
        run(Command::new("git").args(["rev-parse", "HEAD"]))?.trim()
            == receipt["source_revision"]
                .as_str()
                .ok_or("missing revision")?,
        "check out recorded revision in separate worktree",
    )?;
    for record in receipt["results"].as_array().ok_or("missing results")? {
        let b = &record["behavior"];
        let output = run(Command::new(&binary).args([
            "worker",
            "--seed",
            &b["seed"].to_string(),
            "--kind",
            b["kind"].as_str().ok_or("missing kind")?,
            "--protocol",
            "experiments/workshop-v1.json",
        ]))?;
        let replay: Value = serde_json::from_str(&output)?;
        require(
            replay["behavior_hash"] == record["behavior_hash"],
            "behavior mismatch",
        )?;
    }
    println!("Reproduced deterministic worker results");
    Ok(())
}
fn main() -> Result {
    std::env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("knowledge")=>knowledge(), Some("configure-github")=>configure(), Some("issues")=>issues(), Some("reproduce")=>reproduce(&args[1..]),
        Some("formal")=>{let lean=std::env::var("LEAN").unwrap_or_else(|_|"lean".into()); print!("{}",run(Command::new(lean).arg("formal/SI.lean"))?); print!("{}",run(Command::new("cargo").args(["test","--locked","-p","tesseract-si"]))?); Ok(())},
        _=>Err("usage: cargo xtask knowledge|formal|configure-github|issues|reproduce RECEIPT [BINARY] [SOURCE_DIR]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_validation_handles_leap_centuries() {
        assert!(valid_date("2000-02-29"));
        assert!(!valid_date("1900-02-29"));
        assert!(!valid_date("2026-04-31"));
        assert!(!valid_date("2026-00-10"));
        assert!(!valid_date("2026-1-001"));
    }
    #[test]
    fn knowledge_rejects_broken_links_and_false_ratification() {
        let dir = tempfile::tempdir().unwrap();
        for folder in ["decisions", "observations", "research"] {
            fs::create_dir_all(dir.path().join(format!("knowledge/{folder}"))).unwrap();
        }
        fs::write(dir.path().join("knowledge/README.md"), "O-0001").unwrap();
        let path = dir.path().join("knowledge/observations/O-0001-test.md");
        let record = "---\nid: O-0001\ntype: observation\nstatus: test\nupdated: 2026-11-09\nadded_by: agent\nhuman_ratified: false\n---\n## Observed\n## Inferred\n## Hypothesis\n## Unknown\n## Relations\n- *extends*: [index](../README.md)\n## Revisit when\n";
        fs::write(&path, record).unwrap();
        knowledge_at(dir.path()).unwrap();
        fs::write(&path, record.replace("../README.md", "missing.md")).unwrap();
        assert!(knowledge_at(dir.path()).is_err());
        fs::write(
            &path,
            record.replace("human_ratified: false", "human_ratified: true"),
        )
        .unwrap();
        assert!(knowledge_at(dir.path()).is_err());
        fs::write(
            &path,
            record.replace(
                "human_ratified: false",
                "human_ratified: true\nratified_by: \nratified_on: 2026-11-09",
            ),
        )
        .unwrap();
        assert!(knowledge_at(dir.path()).is_err());
    }
}
