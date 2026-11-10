use super::*;

fn noun(text: &str) -> Option<Concept> {
    let concept = Concept::new(text).ok()?;
    if concept.0.split_whitespace().next().is_some_and(|w| {
        matches!(
            w,
            "it" | "this"
                | "these"
                | "those"
                | "they"
                | "he"
                | "she"
                | "we"
                | "you"
                | "our"
                | "their"
                | "its"
        )
    }) {
        return None;
    }
    if concept.0.split_whitespace().any(|w| {
        matches!(
            w,
            "is" | "are"
                | "was"
                | "were"
                | "can"
                | "cannot"
                | "not"
                | "if"
                | "unless"
                | "when"
                | "may"
                | "might"
                | "could"
                | "would"
                | "should"
                | "must"
                | "alone"
                | "sometimes"
                | "usually"
                | "always"
                | "often"
                | "typically"
                | "under"
                | "provided"
                | "except"
                | "without"
                | "during"
                | "after"
                | "before"
                | "neither"
                | "nor"
                | "generally"
                | "possibly"
                | "potentially"
                | "either"
                | "or"
                | "some"
                | "all"
                | "any"
                | "every"
                | "no"
                | "only"
                | "that"
                | "which"
                | "whose"
                | "will"
                | "has"
                | "have"
                | "believed"
                | "proposed"
                | "considered"
                | "but"
        )
    }) {
        return None;
    }
    if concept.0.contains(['"', ':', '(', ')', ',']) {
        return None;
    }
    Some(concept)
}
fn subject(text: &str) -> Option<(Concept, Scope)> {
    match text.strip_suffix(" alone") {
        Some(base) => Some((noun(base)?, Scope::Alone)),
        None => Some((noun(text)?, Scope::Unqualified)),
    }
}
fn relation(
    subject: Concept,
    predicate: Relation,
    object: Concept,
    scope: Scope,
    positive: bool,
) -> Claim {
    Claim {
        atom: Atom::Relation {
            subject,
            predicate,
            object,
            scope,
        },
        positive,
        according_to: None,
    }
}
fn closure(body: &str, attribution: Option<Concept>) -> Option<Claim> {
    let body = body.strip_prefix("linear combinations of solutions to ")?;
    let (left, right) = body.split_once(" are also solutions of ")?;
    let left = noun(left)?;
    if left != noun(right)? {
        return None;
    }
    Some(Claim {
        atom: Atom::ClosedUnderLinearCombination { equation: left },
        positive: true,
        according_to: attribution,
    })
}
fn simple(sentence: &str) -> Vec<(Claim, &'static str)> {
    let mut result = Vec::new();
    if let Some(claim) = closure(sentence, None) {
        result.push((claim, "explicit_linear_closure"));
        return result;
    }
    for (delimiter, predicate, positive) in [
        (" cannot be used for ", Relation::UsableFor, false),
        (" can be used for ", Relation::UsableFor, true),
        (" cannot be used to ", Relation::UsableTo, false),
        (" can be used to ", Relation::UsableTo, true),
        (" can produce ", Relation::CanProduce, true),
        (" cannot produce ", Relation::CanProduce, false),
        (" describes ", Relation::Describes, true),
    ] {
        if let Some((left, right)) = sentence.split_once(delimiter)
            && let (Some((subject, scope)), Some(object)) = (subject(left), noun(right))
        {
            result.push((
                relation(subject, predicate, object, scope, positive),
                "explicit_relation",
            ));
            return result;
        }
    }
    let copula = sentence
        .split_once(" is ")
        .or_else(|| sentence.split_once(" are "));
    let Some((left, right)) = copula else {
        return result;
    };
    let Some(entity) = noun(left) else {
        return result;
    };
    let (positive, right) = right
        .strip_prefix("not ")
        .map_or((true, right), |s| (false, s));
    if let Some(alias) = right.strip_prefix("also known as ").and_then(noun) {
        result.push((
            relation(entity, Relation::Alias, alias, Scope::Unqualified, positive),
            "explicit_alias",
        ));
        return result;
    }
    if let Some(object) = right.strip_prefix("the loss of ").and_then(noun) {
        result.push((
            relation(
                entity,
                Relation::LossOf,
                object,
                Scope::Unqualified,
                positive,
            ),
            "loss_definition",
        ));
        return result;
    }
    // A definition can classify its subject without treating the class as a
    // formal subclass axiom. Additional clauses are interpreted independently.
    let (kind, tail) = right
        .split_once(" that ")
        .map_or((right, None), |(k, t)| (k, Some(t)));
    if !positive && tail.is_some() {
        return result;
    }
    if let Some(kind) = noun(kind) {
        result.push((
            relation(
                entity.clone(),
                Relation::ClassifiedAs,
                kind,
                Scope::Unqualified,
                positive,
            ),
            "definition_category",
        ));
        if positive
            && let Some(body) = tail.and_then(|t| t.strip_prefix("states that "))
            && let Some(claim) = closure(body, Some(entity.clone()))
        {
            result.push((claim, "attributed_linear_closure"));
        }
        if positive
            && let Some(object) = tail
                .and_then(|t| t.strip_prefix("describes "))
                .and_then(noun)
        {
            result.push((
                relation(
                    entity,
                    Relation::Describes,
                    object,
                    Scope::Unqualified,
                    true,
                ),
                "definition_describes",
            ));
        }
    }
    result
}
fn extract(sentence: &str) -> Vec<(Claim, &'static str)> {
    if sentence.trim_end().ends_with('?') {
        return vec![];
    }
    let sentence = sentence
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .to_lowercase();
    if let Some((premise, consequence)) = sentence.split_once(", so ") {
        if premise.split_whitespace().any(|w| {
            matches!(
                w,
                "if" | "unless"
                    | "may"
                    | "might"
                    | "could"
                    | "would"
                    | "sometimes"
                    | "assume"
                    | "suppose"
                    | "believed"
                    | "thought"
                    | "said"
                    | "claimed"
                    | "argued"
                    | "proposed"
                    | "suggested"
                    | "considered"
                    | "according"
            )
        }) {
            return vec![];
        }
        return simple(consequence)
            .into_iter()
            .map(|(claim, _)| (claim, "asserted_consequence"))
            .collect();
    }
    if let Some((left, tail)) = sentence.split_once(", also known as ") {
        if let Some((alias, rest)) = tail.split_once(", ")
            && rest.starts_with("is ")
            && let (Some(entity), Some(alias)) = (noun(left), noun(alias))
        {
            let mut result = vec![(
                relation(entity, Relation::Alias, alias, Scope::Unqualified, true),
                "parenthetical_alias",
            )];
            result.extend(simple(&format!("{left} {rest}")));
            return result;
        }
        return vec![];
    }
    if let Some((first, second)) = sentence.split_once(", but it ") {
        if let Some((left, _)) = first
            .split_once(" can ")
            .or_else(|| first.split_once(" cannot "))
            && subject(left).is_some()
        {
            let mut result = simple(first);
            if result.is_empty() {
                return result;
            }
            result.extend(
                simple(&format!("{left} {second}"))
                    .into_iter()
                    .map(|(claim, _)| (claim, "coordinated_subject")),
            );
            return result;
        }
        return vec![];
    }
    simple(&sentence)
}
pub(super) fn interpret(corpus: &Corpus) -> Result<Interpretation> {
    let mut graph = Interpretation {
        extractor: EXTRACTOR.into(),
        corpus_digest: digest(corpus)?,
        facts: vec![],
        sentences: 0,
        sentences_with_atoms: 0,
    };
    for (a, article) in corpus.articles.iter().enumerate() {
        for (p, passage) in article.passages.iter().enumerate() {
            let mut offset = 0;
            for raw in passage.text.split_inclusive(['.', '!', '?']) {
                let sentence = raw.trim();
                let start = offset + raw.len() - raw.trim_start().len();
                offset += raw.len();
                if sentence.is_empty() {
                    continue;
                }
                graph.sentences += 1;
                if graph.sentences > 32768 {
                    return Err(Error::Budget(
                        "symbolic reader exceeds 32768 sentences".into(),
                    ));
                }
                let claims = extract(sentence);
                if !claims.is_empty() {
                    graph.sentences_with_atoms += 1;
                }
                for (claim, rule) in claims {
                    claim.validate()?;
                    if graph.facts.len() >= MAX_FACTS {
                        return Err(Error::Budget("symbolic reader exceeds 4096 facts".into()));
                    }
                    graph.facts.push(Fact {
                        id: graph.facts.len(),
                        claim,
                        extraction_rule: rule.into(),
                        source: Witness {
                            article: a,
                            passage: p,
                            start,
                            end: start + sentence.len(),
                            revision: article.revision,
                            url: article.url(),
                        },
                    });
                }
            }
        }
    }
    if serde_json::to_vec(&graph)?.len() > 4 * 1024 * 1024 {
        return Err(Error::Budget("symbolic graph exceeds 4 MiB".into()));
    }
    Ok(graph)
}
