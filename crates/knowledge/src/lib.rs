//! Persistent, source-attributed declarative reading memory.
//! Retrieval returns evidence, never an entailment or scientific-truth certificate.
pub mod symbolic;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    path::Path,
};
use tesseract_contracts::{Error, Result, digest};

pub const LICENSE: &str = "https://creativecommons.org/licenses/by-sa/4.0/";
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARTICLES: usize = 64;

pub fn html_digest(html: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(html.as_bytes()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub section: String,
    pub text: String,
    pub equations: Vec<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Article {
    pub page_id: u64,
    pub title: String,
    pub revision: u64,
    pub revision_timestamp: String,
    pub retrieved_at: String,
    pub html_sha256: String,
    pub passages: Vec<Passage>,
    /// Source presentation MathML, retained without interpreting its mathematics.
    pub equations: Vec<String>,
}
impl Article {
    pub fn url(&self) -> String {
        format!(
            "https://en.wikipedia.org/w/index.php?oldid={}",
            self.revision
        )
    }
    pub fn history_url(&self) -> String {
        format!(
            "https://en.wikipedia.org/w/index.php?curid={}&action=history",
            self.page_id
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: u32,
    pub license: String,
    pub articles: Vec<Article>,
}
impl Corpus {
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1
            || self.license != LICENSE
            || self.articles.is_empty()
            || self.articles.len() > MAX_ARTICLES
        {
            return Err(Error::Invalid("unsupported or empty reading corpus".into()));
        }
        let mut ids = BTreeSet::new();
        let mut titles = BTreeSet::new();
        let mut revisions = BTreeSet::new();
        for article in &self.articles {
            if article.page_id == 0
                || article.revision == 0
                || !ids.insert(article.page_id)
                || !revisions.insert(article.revision)
                || !titles.insert(&article.title)
                || article.title.is_empty()
                || article.title.len() > 256
                || article.title.chars().any(char::is_control)
                || !is_hash(&article.html_sha256)
                || article.passages.is_empty()
                || article.passages.len() > 2048
                || article.equations.len() > 4096
                || !timestamp(&article.revision_timestamp)
                || !timestamp(&article.retrieved_at)
            {
                return Err(Error::Invalid(
                    "invalid source provenance or duplicate article".into(),
                ));
            }
            for passage in &article.passages {
                if passage.equations.len() > article.equations.len() {
                    return Err(Error::Invalid("too many equation references".into()));
                }
                let references: BTreeSet<_> = passage.equations.iter().copied().collect();
                let mut markers = BTreeSet::new();
                for tail in passage.text.split("[equation:").skip(1) {
                    let index = tail
                        .split_once(']')
                        .and_then(|(index, _)| index.parse::<usize>().ok())
                        .ok_or_else(|| Error::Invalid("invalid equation marker".into()))?;
                    markers.insert(index);
                }
                if references.len() != passage.equations.len() || references != markers {
                    return Err(Error::Invalid(
                        "equation references must be unique and match markers".into(),
                    ));
                }
                if passage.text.trim().is_empty()
                    || passage.text.len() > 16_384
                    || passage.section.len() > 512
                    || passage
                        .equations
                        .iter()
                        .any(|i| *i >= article.equations.len())
                    || passage
                        .equations
                        .iter()
                        .any(|i| !passage.text.contains(&format!("[equation:{i}]")))
                    || passage
                        .text
                        .chars()
                        .any(|c| c.is_control() && !c.is_whitespace())
                {
                    return Err(Error::Invalid("invalid reading passage".into()));
                }
            }
            for equation in &article.equations {
                if equation.len() > 131_072
                    || !equation.starts_with("<math")
                    || !equation.ends_with("</math>")
                {
                    return Err(Error::Invalid("invalid source MathML".into()));
                }
            }
        }
        if serde_json::to_vec(self)?.len() as u64 > MAX_BYTES / 2 {
            return Err(Error::Budget("reading corpus exceeds 8 MiB".into()));
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let corpus: Self = serde_json::from_slice(&read_bounded(path, MAX_BYTES)?)?;
        corpus.validate()?;
        Ok(corpus)
    }
}
fn timestamp(s: &str) -> bool {
    s.len() == 20 && s.is_ascii() && s.as_bytes()[10] == b'T' && s.ends_with('Z')
}
fn is_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    if limit > MAX_BYTES {
        return Err(Error::Budget("input limit exceeds 16 MiB".into()));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Budget(format!("input exceeds {limit} bytes")));
    }
    Ok(bytes)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema: u32,
    corpus_digest: String,
    corpus: Corpus,
    #[serde(default)]
    interpretation: Option<serde_json::Value>,
    #[serde(default)]
    interpretation_digest: Option<String>,
}
#[derive(Debug, Clone)]
struct IndexedPassage {
    article: usize,
    passage: usize,
    words: BTreeMap<String, usize>,
    length: usize,
}
/// No mutable query cache: the same memory and question produce the same evidence.
pub struct KnowledgeBase {
    corpus: Corpus,
    digest: String,
    index: Vec<IndexedPassage>,
    frequencies: BTreeMap<String, usize>,
    average_length: f64,
    interpretation: Option<symbolic::Interpretation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadingStats {
    pub articles: usize,
    pub passages: usize,
    pub vocabulary: usize,
    pub equations: usize,
    pub corpus_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    pub article: String,
    pub revision: u64,
    pub url: String,
    pub history_url: String,
    pub section: String,
    pub passage: usize,
    pub passage_digest: String,
    pub excerpt: String,
    /// Lexical relevance only; not a probability or truth confidence.
    pub relevance: f64,
    pub equations: Vec<SourceEquation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEquation {
    pub index: usize,
    pub mathml: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnswerStatus {
    EvidenceFound,
    InsufficientEvidence,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Answer {
    pub question: String,
    pub status: AnswerStatus,
    pub corpus_digest: String,
    pub matched_term_fraction: f64,
    pub unmatched_terms: Vec<String>,
    pub evidence: Vec<Evidence>,
}
impl Answer {
    pub fn to_english(&self) -> String {
        if self.status == AnswerStatus::InsufficientEvidence {
            return "I do not have enough relevant evidence in my reading to answer this question."
                .into();
        }
        let mut text = String::from(
            "Relevant passages from my reading (these do not verify the question's premise):\n",
        );
        for (i, evidence) in self.evidence.iter().enumerate() {
            text.push_str(&format!(
                "\n[{}] {}\nSource: {} — {}\n{}\n",
                i + 1,
                evidence.excerpt,
                evidence.article,
                evidence.section,
                evidence.url
            ));
        }
        if !self.unmatched_terms.is_empty() {
            text.push_str(&format!(
                "\nTerms without matches: {}.\n",
                self.unmatched_terms.join(", ")
            ));
        }
        if self.evidence.iter().any(|e| !e.equations.is_empty()) {
            text.push_str("\nEquation references preserve source MathML in memory; see the cited article for typeset notation.\n");
        }
        text
    }
}
impl KnowledgeBase {
    pub fn read(corpus: Corpus) -> Result<Self> {
        corpus.validate()?;
        let interpretation = Some(symbolic::Interpretation::derive(&corpus)?);
        Self::index(corpus, interpretation)
    }
    fn index(corpus: Corpus, interpretation: Option<symbolic::Interpretation>) -> Result<Self> {
        corpus.validate()?;
        let corpus_digest = digest(&corpus)?;
        let mut index = Vec::new();
        let mut frequencies = BTreeMap::new();
        for (a, article) in corpus.articles.iter().enumerate() {
            for (p, passage) in article.passages.iter().enumerate() {
                let mut words = BTreeMap::new();
                for word in tokens(&passage.text) {
                    *words.entry(word).or_insert(0) += 1;
                }
                for word in words.keys() {
                    *frequencies.entry(word.clone()).or_insert(0) += 1;
                }
                let length = words.values().sum();
                index.push(IndexedPassage {
                    article: a,
                    passage: p,
                    words,
                    length,
                });
            }
        }
        let average_length =
            (index.iter().map(|p| p.length).sum::<usize>() as f64 / index.len() as f64).max(1.0);
        Ok(Self {
            corpus,
            digest: corpus_digest,
            index,
            frequencies,
            average_length,
            interpretation,
        })
    }
    pub fn stats(&self) -> ReadingStats {
        ReadingStats {
            articles: self.corpus.articles.len(),
            passages: self.index.len(),
            vocabulary: self.frequencies.len(),
            equations: self.corpus.articles.iter().map(|a| a.equations.len()).sum(),
            corpus_digest: self.digest.clone(),
        }
    }
    pub fn corpus(&self) -> &Corpus {
        &self.corpus
    }
    pub fn interpretation(&self) -> Result<&symbolic::Interpretation> {
        self.interpretation.as_ref().ok_or_else(|| {
            Error::Invalid(
                "legacy memory has no symbolic interpretation; read its corpus into a new memory"
                    .into(),
            )
        })
    }
    /// Exclusive atomic publication; existing memories are never replaced.
    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        let bytes = serde_json::to_vec(&Snapshot {
            schema: if self.interpretation.is_some() { 2 } else { 1 },
            corpus_digest: self.digest.clone(),
            corpus: self.corpus.clone(),
            interpretation: self
                .interpretation
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
            interpretation_digest: self
                .interpretation
                .as_ref()
                .map(symbolic::Interpretation::digest)
                .transpose()?,
        })?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(Error::Budget("reading snapshot exceeds 16 MiB".into()));
        }
        file.as_file_mut().write_all(&bytes)?;
        file.as_file_mut().flush()?;
        file.as_file().sync_all()?;
        file.persist_noclobber(path)
            .map_err(|e| Error::Io(e.error))?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
    pub fn open(path: &Path) -> Result<Self> {
        let snapshot: Snapshot = serde_json::from_slice(&read_bounded(path, MAX_BYTES)?)?;
        if ![1, 2].contains(&snapshot.schema)
            || !is_hash(&snapshot.corpus_digest)
            || digest(&snapshot.corpus)? != snapshot.corpus_digest
        {
            return Err(Error::Corrupt("reading snapshot digest mismatch".into()));
        }
        snapshot.corpus.validate()?;
        let interpretation = if snapshot.schema == 2 {
            let stored = snapshot
                .interpretation
                .ok_or_else(|| Error::Corrupt("missing symbolic interpretation".into()))?;
            if stored["extractor"] != symbolic::EXTRACTOR {
                return Err(Error::Invalid(
                    "unsupported extractor version; reinterpret corpus explicitly".into(),
                ));
            }
            let graph = symbolic::Interpretation::derive(&snapshot.corpus)?;
            if snapshot.interpretation_digest.as_deref() != Some(graph.digest()?.as_str())
                || stored != serde_json::to_value(&graph)?
            {
                return Err(Error::Corrupt(
                    "stored interpretation differs from its source-bound derivation".into(),
                ));
            }
            Some(graph)
        } else {
            if snapshot.interpretation.is_some() || snapshot.interpretation_digest.is_some() {
                return Err(Error::Corrupt(
                    "legacy schema cannot contain an interpretation".into(),
                ));
            }
            None
        };
        Self::index(snapshot.corpus, interpretation)
    }
    pub fn ask(&self, question: &str) -> Result<Answer> {
        validate_question(question)?;
        let query: BTreeSet<_> = tokens(question).into_iter().collect();
        let unmatched: Vec<_> = query
            .iter()
            .filter(|w| !self.frequencies.contains_key(*w))
            .cloned()
            .collect();
        let matched = query.len() - unmatched.len();
        let fraction = matched as f64 / query.len().max(1) as f64;
        let mut answer = Answer {
            question: question.into(),
            status: AnswerStatus::InsufficientEvidence,
            corpus_digest: self.digest.clone(),
            matched_term_fraction: fraction,
            unmatched_terms: unmatched,
            evidence: Vec::new(),
        };
        // This rejects unrelated questions even if they happen to say "quantum".
        // It is a relevance gate, not entailment or contradiction detection.
        if matched == 0 || fraction < 0.65 {
            return Ok(answer);
        }
        let n = self.index.len() as f64;
        let mut ranked = Vec::new();
        let titles: Vec<BTreeSet<_>> = self
            .corpus
            .articles
            .iter()
            .map(|a| tokens(&a.title).into_iter().collect())
            .collect();
        for (i, passage) in self.index.iter().enumerate() {
            if passage.length < 4 {
                continue;
            }
            let mut score = 0.0;
            let mut matches = 0;
            for word in &query {
                if let Some(tf) = passage.words.get(word) {
                    matches += 1;
                    let df = self.frequencies[word] as f64;
                    let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
                    let tf = *tf as f64;
                    score += idf * tf * 2.2
                        / (tf + 1.2 * (0.25 + 0.75 * passage.length as f64 / self.average_length));
                }
            }
            // Corpus-wide vocabulary coverage alone admits unrelated questions
            // whose words occur in unrelated places. Every returned passage must
            // cover at least half of the content terms in this specific question.
            if matches > 0 && matches * 2 >= query.len() {
                score *= matches as f64 / matched as f64;
                let title = &titles[passage.article];
                // Title priors do not make tiny fragments appear highly relevant.
                // Definitions belong to the exact topic, not every title that mentions it.
                if !title.is_empty() && title.is_subset(&query) {
                    score += 2.0;
                    if passage.passage == 0
                        && (question.to_lowercase().starts_with("what ") || *title == query)
                    {
                        score += 10.0;
                    }
                }
                ranked.push((i, score));
            }
        }
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        // A small evidence bundle, with at most two passages from any article.
        let mut counts = BTreeMap::new();
        for (i, score) in ranked {
            let entry = &self.index[i];
            let count = counts.entry(entry.article).or_insert(0);
            if *count >= 2 {
                continue;
            }
            *count += 1;
            let article = &self.corpus.articles[entry.article];
            let passage = &article.passages[entry.passage];
            answer.evidence.push(Evidence {
                article: article.title.clone(),
                revision: article.revision,
                url: article.url(),
                history_url: article.history_url(),
                section: passage.section.clone(),
                passage: entry.passage,
                passage_digest: digest(passage)?,
                excerpt: passage.text.clone(),
                relevance: score,
                equations: passage
                    .equations
                    .iter()
                    .map(|i| SourceEquation {
                        index: *i,
                        mathml: article.equations[*i].clone(),
                    })
                    .collect(),
            });
            if answer.evidence.len() == 3 {
                break;
            }
        }
        if !answer.evidence.is_empty() {
            answer.status = AnswerStatus::EvidenceFound;
        }
        Ok(answer)
    }
}
pub fn validate_question(question: &str) -> Result<()> {
    if question.trim().is_empty() || question.len() > 2048 || question.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "question must contain 1–2048 bytes without control characters".into(),
        ));
    }
    Ok(())
}
/// Small, general normalization vocabulary; there are no stored question/answer pairs.
fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|word| {
            if word.len() < 2
                || matches!(
                    word,
                    "the"
                        | "of"
                        | "to"
                        | "in"
                        | "is"
                        | "are"
                        | "was"
                        | "were"
                        | "be"
                        | "been"
                        | "being"
                        | "for"
                        | "and"
                        | "or"
                        | "an"
                        | "at"
                        | "on"
                        | "as"
                        | "by"
                        | "it"
                        | "its"
                        | "this"
                        | "that"
                        | "these"
                        | "those"
                        | "with"
                        | "from"
                        | "what"
                        | "which"
                        | "how"
                        | "why"
                        | "who"
                        | "when"
                        | "where"
                        | "can"
                        | "cannot"
                        | "does"
                        | "do"
                        | "did"
                        | "we"
                        | "you"
                        | "my"
                        | "me"
                        | "have"
                        | "has"
                        | "had"
                        | "not"
                        | "than"
                        | "then"
                        | "so"
                        | "if"
                        | "into"
                        | "about"
                        | "explain"
                        | "tell"
                        | "say"
                        | "mean"
                        | "difference"
                        | "between"
                        | "happens"
                        | "work"
                        | "works"
                        | "know"
                        | "together"
                        | "quantum"
                )
            {
                return None;
            }
            let canonical = match word {
                "schrödinger" => "schrodinger",
                "entangled" | "entangling" | "entangle" => "entanglement",
                "measure" | "measuring" | "measurements" | "observing" | "observation"
                | "observe" => "measurement",
                "message" | "messages" | "communicate" | "communicating" | "signal" | "signals"
                | "signalling" | "signaling" | "send" | "sending" => "communication",
                "tunneling" | "tunnelling" | "tunnels" => "tunnel",
                "forbidden" | "forbids" => "forbid",
                "physics" | "physical" => "physic",
                "describes" | "describe" => "description",
                "affect" | "affects" | "changes" | "changing" | "changed" => "change",
                _ => word
                    .strip_suffix('s')
                    .filter(|_| word.len() > 4 && !word.ends_with("ss"))
                    .unwrap_or(word),
            };
            Some(canonical.to_owned())
        })
        .collect()
}
