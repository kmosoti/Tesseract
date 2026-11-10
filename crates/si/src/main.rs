use std::error::Error;
use std::path::Path;
use tesseract_contracts::Budget;
use tesseract_journal::Session;
use tesseract_knowledge::{Corpus, KnowledgeBase};
use tesseract_si::{Term, english::from_english};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["read",corpus,memory]=>{
            let knowledge=KnowledgeBase::read(Corpus::load(Path::new(corpus))?)?;
            knowledge.save(Path::new(memory))?;
            println!("{}",serde_json::to_string_pretty(&knowledge.stats())?);
        },
        ["ask-knowledge",memory,question]=>{
            let knowledge=KnowledgeBase::open(Path::new(memory))?;
            let term=Term::Reason(tesseract_knowledge::symbolic::language::compile(question)?);
            let answer=term.evaluate_symbolic(&knowledge)?;
            println!("SI: {}\n{}",term.to_source()?,tesseract_knowledge::symbolic::language::render(&answer));
        },
        ["inspect-symbols",memory]=>{
            let knowledge=KnowledgeBase::open(Path::new(memory))?;
            println!("{}",serde_json::to_string_pretty(&serde_json::json!({"stats":knowledge.interpretation()?.stats()?,"graph":knowledge.interpretation()?}))?);
        },
        ["reason",memory,source]=>{
            let knowledge=KnowledgeBase::open(Path::new(memory))?;
            println!("{}",serde_json::to_string_pretty(&Term::parse(source)?.evaluate_symbolic(&knowledge)?)?);
        },
        ["translate","--science-to-si",question]=>{
            println!("{}",Term::Reason(tesseract_knowledge::symbolic::language::compile(question)?).to_source()?);
        },
        ["retrieve-knowledge",memory,question]=>{
            let knowledge=KnowledgeBase::open(Path::new(memory))?;
            let term=Term::Recall((*question).into());
            let thought=term.evaluate_reading(&knowledge)?;
            println!("SI: {}\n{}",term.to_source()?,thought.answer.to_english());
        },
        ["recall",memory,source]=>{
            let knowledge=KnowledgeBase::open(Path::new(memory))?;
            println!("{}",serde_json::to_string_pretty(&Term::parse(source)?.evaluate_reading(&knowledge)?)?);
        },
        ["translate","--to-si",english]=>println!("{}",from_english(english)?.to_source()?),
        ["translate","--to-english",symbolic]=>println!("{}",Term::parse(symbolic)?.to_english()?),
        ["ask",journal,english]=>{
            let term=from_english(english)?;
            let session=Session::open(journal)?;
            println!("SI: {}\n{}",term.to_source()?,term.evaluate(session.kernel(),Budget::default())?.to_english()?);
        },
        ["explain",journal,symbolic]=>{
            let term=Term::parse(symbolic)?;
            let session=Session::open(journal)?;
            println!("{}",term.evaluate(session.kernel(),Budget::default())?.to_english()?);
        },
        [journal,symbolic]=>{
            let session=Session::open(journal)?;
            let thought=Term::parse(symbolic)?.evaluate(session.kernel(),Budget::default())?;
            println!("{}",serde_json::to_string_pretty(&thought)?);
        },
        _=>return Err("usage: tesseract-si read CORPUS MEMORY | ask-knowledge MEMORY QUESTION | inspect-symbols MEMORY | reason MEMORY SI | retrieve-knowledge MEMORY QUESTION | recall MEMORY SI | translate --science-to-si QUESTION | translate --to-si ENGLISH | translate --to-english SI | ask JOURNAL ENGLISH | explain JOURNAL SI | JOURNAL SI".into())
    }
    Ok(())
}
