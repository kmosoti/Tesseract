use std::error::Error;
use tesseract_contracts::Budget;
use tesseract_journal::Session;
use tesseract_si::Term;
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: tesseract-si JOURNAL 'seek porous 0 2'".into());
    }
    let session = Session::open(&args[0])?;
    let thought = Term::parse(&args[1])?.evaluate(session.kernel(), Budget::default())?;
    println!("{}", serde_json::to_string_pretty(&thought)?);
    Ok(())
}
