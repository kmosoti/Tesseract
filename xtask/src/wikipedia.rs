//! Explicit network import, separate from offline scientific reading.
use crate::{Result, require};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
};
use tesseract_knowledge::{Article, Corpus, LICENSE, Passage};

fn api(params: &[(&str, String)]) -> Result<Value> {
    let mut command = Command::new("curl");
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "45",
        "--max-filesize",
        "8388608",
        "--proto",
        "=https",
        "--user-agent",
        "TesseractScientificReader/0.1 (https://github.com/kmosoti/Tesseract)",
        "--get",
        "https://en.wikipedia.org/w/api.php",
    ]);
    for (key, value) in params {
        command.args(["--data-urlencode", &format!("{key}={value}")]);
    }
    command.args([
        "--data-urlencode",
        "format=json",
        "--data-urlencode",
        "formatversion=2",
    ]);
    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or("missing curl output")?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        child.kill()?;
        child.wait()?;
        return Err("Wikipedia response exceeds 8 MiB".into());
    }
    require(child.wait()?.success(), "Wikipedia request failed")?;
    let value: Value = serde_json::from_slice(&bytes)?;
    require(
        value.get("error").is_none(),
        &format!("Wikipedia API error: {}", value["error"]),
    )?;
    Ok(value)
}
fn excluded(element: ElementRef<'_>) -> bool {
    element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .chain(std::iter::once(element))
        .any(|e| {
            matches!(
                e.value().name(),
                "table" | "figure" | "aside" | "style" | "script" | "math"
            ) || e.value().classes().any(|c| {
                matches!(
                    c,
                    "reference"
                        | "mw-editsection"
                        | "hatnote"
                        | "navbox"
                        | "metadata"
                        | "noprint"
                        | "ambox"
                        | "reflist"
                )
            })
        })
}
/// Keep source mathematics without guessing algebra from presentation markup.
fn text(element: ElementRef<'_>, equations: &mut Vec<String>) -> (String, Vec<usize>) {
    let mut raw = String::new();
    let mut references = Vec::new();
    for node in element.descendants() {
        if let Some(t) = node.value().as_text() {
            if !node.ancestors().filter_map(ElementRef::wrap).any(excluded) {
                raw.push_str(t);
            }
        } else if let Some(e) = ElementRef::wrap(node)
            && e.value().name() == "math"
            && !e.ancestors().filter_map(ElementRef::wrap).any(excluded)
        {
            let index = equations.len();
            equations.push(e.html());
            references.push(index);
            raw.push_str(&format!(" [equation:{index}] "));
        }
    }
    (
        raw.split_whitespace().collect::<Vec<_>>().join(" "),
        references,
    )
}
fn passages(html: &str) -> Result<(Vec<Passage>, Vec<String>)> {
    let doc = Html::parse_fragment(html);
    let selector = Selector::parse("h2,h3,h4,p,math").map_err(|_| "invalid paragraph selector")?;
    let mut section = "Introduction".to_owned();
    let mut result = Vec::new();
    let mut equations = Vec::new();
    let mut appendix = false;
    for element in doc.select(&selector) {
        if element.value().name() == "math" {
            if !appendix
                && !element
                    .ancestors()
                    .filter_map(ElementRef::wrap)
                    .any(|e| e.value().name() == "p" || excluded(e))
            {
                let index = equations.len();
                equations.push(element.html());
                result.push(Passage {
                    section: section.clone(),
                    text: format!("[equation:{index}]"),
                    equations: vec![index],
                });
            }
            continue;
        }
        if excluded(element) {
            continue;
        }
        if appendix && element.value().name() == "p" {
            continue;
        }
        let (value, references) = text(element, &mut equations);
        if element.value().name() != "p" {
            section = value;
            if element.value().name() == "h2" {
                appendix = [
                    "References",
                    "Notes",
                    "Explanatory notes",
                    "See also",
                    "Further reading",
                    "External links",
                    "Bibliography",
                ]
                .contains(&section.as_str());
            }
            continue;
        }
        if appendix || value.len() < 50 {
            continue;
        }
        // Bounded, contiguous normalized-text passages. Never author answers here.
        let words: Vec<_> = value.split_whitespace().collect();
        let first = result.len();
        for chunk in words.chunks(140) {
            let chunk_text = chunk.join(" ");
            let chunk_equations: Vec<_> = references
                .iter()
                .filter(|i| chunk_text.contains(&format!("[equation:{i}]")))
                .copied()
                .collect();
            if chunk.len() < 25 && result.len() > first {
                let previous: &mut Passage =
                    result.last_mut().ok_or("missing preceding passage")?;
                previous.text.push(' ');
                previous.text.push_str(&chunk_text);
                previous.equations.extend(chunk_equations);
                continue;
            }
            result.push(Passage {
                section: section.clone(),
                text: chunk_text,
                equations: chunk_equations,
            });
        }
    }
    require(
        !result.is_empty(),
        "Wikipedia article contains no readable paragraphs",
    )?;
    Ok((result, equations))
}
pub fn import(args: &[String]) -> Result {
    let (topics_path, output_path, archive) = match args {
        [topics, output] => (topics, output, None),
        [topics, output, archive] => (topics, output, Some(Path::new(archive))),
        _ => {
            return Err(
                "usage: cargo xtask wikipedia TOPICS.json CORPUS.json [RAW_HTML_DIR]".into(),
            );
        }
    };
    require(
        !Path::new(output_path).exists(),
        "corpus output already exists",
    )?;
    let topics: Vec<String> = serde_json::from_slice(&tesseract_knowledge::read_bounded(
        Path::new(topics_path),
        65_536,
    )?)?;
    if let Some(archive) = archive {
        fs::create_dir(archive)?;
    }
    require(
        !topics.is_empty() && topics.len() <= 64,
        "expected 1–64 article titles",
    )?;
    let mut articles = Vec::new();
    for title in topics {
        require(
            !title.is_empty() && title.len() <= 256 && !title.contains('|'),
            "invalid Wikipedia title",
        )?;
        let metadata = api(&[
            ("action", "query".into()),
            ("prop", "revisions".into()),
            ("rvprop", "ids|timestamp".into()),
            ("redirects", "1".into()),
            ("curtimestamp", "1".into()),
            ("titles", title),
        ])?;
        let page = &metadata["query"]["pages"][0];
        let revision = page["revisions"][0]["revid"]
            .as_u64()
            .ok_or("missing source revision")?;
        let parsed = api(&[
            ("action", "parse".into()),
            ("oldid", revision.to_string()),
            ("prop", "text|revid".into()),
        ])?;
        require(
            parsed["parse"]["revid"].as_u64() == Some(revision),
            "Wikipedia returned wrong revision",
        )?;
        let html = parsed["parse"]["text"]
            .as_str()
            .ok_or("missing revision HTML")?;
        if let Some(archive) = archive {
            let mut raw = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(archive.join(format!("{revision}.html")))?;
            raw.write_all(html.as_bytes())?;
            raw.sync_all()?;
        }
        let (passages, equations) = passages(html)?;
        let article = Article {
            page_id: page["pageid"].as_u64().ok_or("missing page ID")?,
            title: page["title"].as_str().ok_or("missing title")?.into(),
            revision,
            revision_timestamp: page["revisions"][0]["timestamp"]
                .as_str()
                .ok_or("missing revision timestamp")?
                .into(),
            retrieved_at: metadata["curtimestamp"]
                .as_str()
                .ok_or("missing retrieval timestamp")?
                .into(),
            html_sha256: tesseract_knowledge::html_digest(html),
            passages,
            equations,
        };
        println!(
            "Read {} @ {}: {} passages",
            article.title,
            revision,
            article.passages.len()
        );
        articles.push(article);
    }
    let corpus = Corpus {
        schema: 1,
        license: LICENSE.into(),
        articles,
    };
    corpus.validate()?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    file.write_all(&serde_json::to_vec_pretty(&corpus)?)?;
    file.sync_all()?;
    Ok(())
}
pub fn verify(args: &[String]) -> Result {
    let [corpus_path, raw_dir] = args else {
        return Err("usage: cargo xtask wikipedia-verify CORPUS RAW_HTML_DIR".into());
    };
    let corpus = Corpus::load(Path::new(corpus_path))?;
    for article in &corpus.articles {
        let raw = tesseract_knowledge::read_bounded(
            &Path::new(raw_dir).join(format!("{}.html", article.revision)),
            8 * 1024 * 1024,
        )?;
        let html = String::from_utf8(raw)?;
        require(
            tesseract_knowledge::html_digest(&html) == article.html_sha256,
            "archived HTML digest mismatch",
        )?;
        let (prose, math) = passages(&html)?;
        require(
            prose == article.passages && math == article.equations,
            "corpus differs from archived source extraction",
        )?;
    }
    println!(
        "Verified {} source HTML digests and complete prose/MathML extraction",
        corpus.articles.len()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extractor_keeps_prose_and_math_but_excludes_navigation_and_references() {
        let html = r#"<div><table><tr><td><p>This unwanted navigation paragraph must never become a scientific claim.</p></td></tr></table><p>A quantum state has <b>amplitudes</b> and observable probabilities.<sup class="reference">[99]</sup> Here is <span><math><mi>ψ</mi></math><img class="mwe-math-fallback-image-inline" alt="ψ"/></span>.</p><h2>References</h2><p>This is an unwanted references paragraph with enough characters to be indexed.</p></div>"#;
        let (result, equations) = passages(html).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].text.contains("amplitudes and observable"));
        assert!(result[0].text.contains("[equation:0]"));
        assert!(!result[0].text.contains("99"));
        assert_eq!(equations.len(), 1);
        assert!(equations[0].contains("<mi>ψ</mi>"));
    }
    #[test]
    fn native_math_preserves_source_structure_instead_of_interpreting_scripts() {
        let html = r#"<p>The uncertainty bound includes <math><mfrac><mi>ℏ</mi><mn>2</mn></mfrac></math> and this is relevant scientific prose.</p><math><msup><mi>ψ</mi><mn>2</mn></msup></math>"#;
        let (result, equations) = passages(html).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].text.contains("[equation:0]"));
        assert_eq!(result[1].text, "[equation:1]");
        assert!(equations[0].contains("<mfrac><mi>ℏ</mi><mn>2</mn></mfrac>"));
        assert!(equations[1].contains("<msup><mi>ψ</mi><mn>2</mn></msup>"));
        let actual = r#"<p>Absolute values and integral limits require presentation-aware treatment: <math><mo>|</mo><mi>α</mi><msup><mo>|</mo><mn>2</mn></msup></math> and <math><msubsup><mo>∫</mo><mi>a</mi><mi>b</mi></msubsup><mi>f</mi></math>.</p>"#;
        let (prose, math) = passages(actual).unwrap();
        assert_eq!(prose[0].equations, vec![0, 1]);
        assert!(math[0].contains("<msup><mo>|</mo><mn>2</mn></msup>"));
        assert!(math[1].contains("<msubsup><mo>∫</mo><mi>a</mi><mi>b</mi></msubsup>"));
        assert!(!prose[0].text.contains("(|)"));
    }
    #[test]
    fn oversized_topics_fail_before_json_parsing_or_network_access() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("topics.json");
        fs::File::create(&input).unwrap().set_len(65_537).unwrap();
        let err = import(&[
            input.to_string_lossy().into(),
            dir.path().join("out.json").to_string_lossy().into(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("resource budget exceeded"));
    }
}
