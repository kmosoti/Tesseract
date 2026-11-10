use std::error::Error;
use tesseract_contracts::Budget;
use tesseract_journal::Session;
use tesseract_si::{Term, english::from_english};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
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
        _=>return Err("usage: tesseract-si translate --to-si ENGLISH | translate --to-english SI | ask JOURNAL ENGLISH | explain JOURNAL SI | JOURNAL SI".into())
    }
    Ok(())
}
