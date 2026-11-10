use crate::{Result, require};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};
use tesseract_knowledge::{KnowledgeBase, MAX_BYTES, read_bounded};

pub fn interview(args: &[String]) -> Result {
    require(
        args.len() == 3 || (args.len() == 4 && args[3] == "--held-out"),
        "usage: cargo xtask reading-interview MEMORY PROTOCOL OUTPUT.json [--held-out]",
    )?;
    let memory_path = Path::new(&args[0]);
    let before = read_bounded(memory_path, MAX_BYTES)?;
    let memory = KnowledgeBase::open(memory_path)?;
    let protocol: Value = serde_json::from_slice(&read_bounded(Path::new(&args[1]), 524_288)?)?;
    let group = if args.len() == 4 {
        "held_out_questions"
    } else {
        "questions"
    };
    let questions = protocol[group]
        .as_array()
        .ok_or("missing interview questions")?;
    require(
        !questions.is_empty() && questions.len() <= 64,
        "expected 1–64 interview questions",
    )?;
    let mut answers = Vec::new();
    let mut transcript = String::new();
    for question in questions {
        let question = question.as_str().ok_or("question must be text")?;
        let answer = memory.ask(question)?;
        require(
            answer == KnowledgeBase::open(memory_path)?.ask(question)?,
            "reopened memory changed answer",
        )?;
        for evidence in &answer.evidence {
            let source = memory
                .corpus()
                .articles
                .iter()
                .find(|a| a.revision == evidence.revision)
                .ok_or("missing cited article")?;
            require(
                source.passages[evidence.passage].text == evidence.excerpt,
                "excerpt not bound to stored passage",
            )?;
            require(
                evidence
                    .equations
                    .iter()
                    .all(|e| source.equations.get(e.index) == Some(&e.mathml)),
                "equation not bound to source markup",
            )?;
        }
        transcript.push_str(&format!(
            "Question: {question}\nTesseract: {}\n\n",
            answer.to_english()
        ));
        answers.push(answer);
    }
    require(
        before == read_bounded(memory_path, MAX_BYTES)?,
        "questions changed persisted reading memory",
    )?;
    let source_commit = crate::run(std::process::Command::new("git").args(["rev-parse", "HEAD"]))?;
    let source_status = crate::run(std::process::Command::new("git").args([
        "status",
        "--porcelain",
        "--",
        "Cargo.toml",
        "Cargo.lock",
        "crates/knowledge",
        "crates/si",
        "xtask",
    ]))?;
    let output = json!({"schema":1,"group":group,"source_commit":source_commit.trim(),"source_tree_clean":source_status.is_empty(),"stats":memory.stats(),"protocol_digest":tesseract_knowledge::html_digest(&serde_json::to_string(&protocol)?),"memory_unchanged":true,"reopen_answers_identical":true,"all_excerpts_source_bound":true,"answers":answers});
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])?;
    file.write_all(&serde_json::to_vec_pretty(&output)?)?;
    let mut text = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(Path::new(&args[2]).with_extension("txt"))?;
    text.write_all(transcript.as_bytes())?;
    println!(
        "Interviewed {} questions; memory unchanged and reopened answers identical",
        questions.len()
    );
    Ok(())
}
