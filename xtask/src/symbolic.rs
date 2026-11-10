use crate::{Result, require};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path, process::Command};
use tesseract_knowledge::{
    KnowledgeBase, MAX_BYTES, read_bounded,
    symbolic::{Premise, Query, language},
};
pub fn interview(args: &[String]) -> Result {
    let [memory_path, protocol_path, output_path] = args else {
        return Err("usage: cargo xtask symbolic-interview MEMORY PROTOCOL OUTPUT.json".into());
    };
    let path = Path::new(memory_path);
    let before = read_bounded(path, MAX_BYTES)?;
    let memory = KnowledgeBase::open(path)?;
    let graph = memory.interpretation()?;
    let protocol: Value = serde_json::from_slice(&read_bounded(Path::new(protocol_path), 262144)?)?;
    let questions = protocol["questions"]
        .as_array()
        .ok_or("missing questions")?;
    require(
        !questions.is_empty() && questions.len() <= 64,
        "expected1–64 questions",
    )?;
    let mut answers = Vec::new();
    let mut transcript = String::new();
    for question in questions {
        let question = question.as_str().ok_or("question is not text")?;
        let query = language::compile(question)?;
        let answer = graph.evaluate(&query)?;
        require(
            answer
                == KnowledgeBase::open(path)?
                    .interpretation()?
                    .evaluate(&query)?,
            "reopened symbolic answer changed",
        )?;
        for fact in &answer.sources {
            require(
                graph.facts().get(fact.id) == Some(fact),
                "proof source differs from acquired graph",
            )?;
            let source =
                &memory.corpus().articles[fact.source.article].passages[fact.source.passage].text;
            require(
                source.get(fact.source.start..fact.source.end).is_some(),
                "invalid witness byte span",
            )?;
        }
        for step in &answer.proof {
            for premise in &step.premises {
                match premise {
                    Premise::Source { fact } => require(
                        answer.sources.iter().any(|f| f.id == *fact),
                        "missing proof witness",
                    )?,
                    Premise::Assumption { index } => require(
                        matches!(&query,Query::Conditional {assumptions,..} if *index<assumptions.len()),
                        "missing conditional assumption",
                    )?,
                }
            }
        }
        transcript.push_str(&format!(
            "Question: {question}\nSI: reason {}\n{}\n",
            serde_json::to_string(&query)?,
            language::render(&answer)
        ));
        answers.push(json!({"question":question,"query":query,"understanding":answer}));
    }
    require(
        before == read_bounded(path, MAX_BYTES)?,
        "symbolic questions changed memory",
    )?;
    let source_commit = crate::run(Command::new("git").args(["rev-parse", "HEAD"]))?;
    let source_status = crate::run(Command::new("git").args([
        "status",
        "--porcelain",
        "--",
        "Cargo.toml",
        "Cargo.lock",
        "crates/knowledge",
        "crates/si",
        "xtask",
        "formal",
    ]))?;
    let report = json!({"schema":1,"source_commit":source_commit.trim(),"source_tree_clean":source_status.is_empty(),"corpus_digest":memory.stats().corpus_digest,"interpretation":graph.stats()?,"memory_unchanged":true,"reopen_answers_identical":true,"all_proof_references_bound":true,"answers":answers});
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output_path)?;
    file.write_all(&serde_json::to_vec_pretty(&report)?)?;
    let mut text = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(Path::new(output_path).with_extension("txt"))?;
    text.write_all(transcript.as_bytes())?;
    println!(
        "{} symbolic queries: memory unchanged, reopened proofs identical, witnesses bound",
        questions.len()
    );
    Ok(())
}
