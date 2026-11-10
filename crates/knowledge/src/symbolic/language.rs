//! Pure syntax conversion and result realization. No corpus, index or engine access.
use super::{Atom, Claim, Concept, Element, Premise, Query, Relation, Scope, Understanding};
use tesseract_contracts::{Error, Result};
fn phrase(text: &str) -> Result<Concept> {
    let concept = Concept::new(text)?;
    if concept.0.split_whitespace().any(|w| {
        matches!(
            w,
            "can"
                | "be"
                | "is"
                | "are"
                | "has"
                | "have"
                | "should"
                | "would"
                | "must"
                | "alone"
                | "not"
                | "cannot"
                | "if"
                | "unless"
                | "when"
                | "may"
                | "might"
                | "could"
                | "sometimes"
                | "usually"
                | "generally"
                | "possibly"
                | "potentially"
                | "often"
                | "typically"
                | "probably"
                | "during"
                | "after"
                | "before"
                | "always"
                | "only"
                | "either"
                | "or"
                | "all"
                | "any"
                | "some"
                | "every"
                | "no"
                | "neither"
                | "nor"
                | "that"
                | "which"
                | "provided"
                | "without"
                | "under"
                | "except"
        )
    }) {
        return Err(Error::Invalid(
            "unsupported scope or clause in concept phrase".into(),
        ));
    }
    Ok(concept)
}
fn claim(atom: Atom) -> Claim {
    Claim {
        atom,
        positive: true,
        according_to: None,
    }
}
fn relation(subject: &str, predicate: Relation, object: &str) -> Result<Query> {
    let (subject, scope) = subject
        .strip_suffix(" alone")
        .map_or((subject, Scope::Unqualified), |s| (s, Scope::Alone));
    Ok(Query::Verify {
        claim: claim(Atom::Relation {
            subject: phrase(subject)?,
            predicate,
            object: phrase(object)?,
            scope,
        }),
    })
}
/// Compiles the question's structure only. It has no memory parameter or answers.
pub fn compile(source: &str) -> Result<Query> {
    crate::validate_question(source)?;
    match compile_inner(source) {
        Err(Error::Invalid(_)) => Ok(Query::Unsupported {
            text: source.into(),
        }),
        other => other,
    }
}
fn compile_inner(source: &str) -> Result<Query> {
    let text = source.trim().trim_end_matches(['?', '.']).to_lowercase();
    if let Some(entity) = text.strip_prefix("what is ") {
        return Ok(Query::Describe {
            entity: phrase(entity)?,
        });
    }
    if let Some(body) = text.strip_prefix("is it false that ") {
        if body.starts_with("is it false that ") {
            return Ok(Query::Unsupported {
                text: source.into(),
            });
        }
        let Some((subject, predicate)) = body.split_once(" can ") else {
            return Ok(Query::Unsupported {
                text: source.into(),
            });
        };
        let base = compile_inner(&format!("can {subject} {predicate}"))?;
        if let Query::Verify { mut claim } = base {
            claim.positive = false;
            return Ok(Query::Verify { claim });
        }
    }
    for (start, delimiter, predicate) in [
        ("can ", " be used for ", Relation::UsableFor),
        ("can ", " be used to ", Relation::UsableTo),
        ("can ", " produce ", Relation::CanProduce),
    ] {
        if let Some((subject, object)) = text
            .strip_prefix(start)
            .and_then(|t| t.split_once(delimiter))
        {
            return relation(subject, predicate, object);
        }
    }
    if let Some(body) = text.strip_prefix("is ") {
        if let Some((left, right)) = body.split_once(" the loss of ") {
            return relation(left, Relation::LossOf, right);
        }
        if let Some((left, right)) = body.split_once(" a ").or_else(|| body.split_once(" an ")) {
            return relation(left, Relation::ClassifiedAs, right);
        }
    }
    if let Some((context, rest)) = text
        .strip_prefix("under ")
        .and_then(|t| t.split_once(", if "))
        && let Some((variables, equation)) = rest
            .strip_suffix(", is their linear combination also a solution")
            .and_then(|t| t.split_once(" are solutions of "))
        && let Some((left, right)) = variables.split_once(" and ")
    {
        if left.split_whitespace().count() != 1 || right.split_whitespace().count() != 1 {
            return Ok(Query::Unsupported {
                text: source.into(),
            });
        }
        let context = Some(phrase(context)?);
        let equation = phrase(equation)?;
        let left = phrase(left)?;
        let right = phrase(right)?;
        let member = |name: Concept| Claim {
            atom: Atom::Solution {
                element: Element::Named { name },
                equation: equation.clone(),
            },
            positive: true,
            according_to: context.clone(),
        };
        let query = Query::Conditional {
            assumptions: vec![member(left.clone()), member(right.clone())],
            conclusion: Claim {
                atom: Atom::Solution {
                    element: Element::LinearCombination { left, right },
                    equation,
                },
                positive: true,
                according_to: context,
            },
        };
        query.validate()?;
        return Ok(query);
    }
    Ok(Query::Unsupported {
        text: source.into(),
    })
}
pub fn display_claim(claim: &Claim) -> String {
    let relation = match &claim.atom {
        Atom::Relation {
            subject,
            predicate,
            object,
            scope,
        } => format!(
            "{:?}({}{}, {})",
            predicate,
            subject.0,
            if *scope == Scope::Alone { " alone" } else { "" },
            object.0
        ),
        Atom::ClosedUnderLinearCombination { equation } => format!(
            "closed_under(solutions({}), linear_combination)",
            equation.0
        ),
        Atom::Solution { element, equation } => {
            let element = match element {
                Element::Named { name } => name.0.clone(),
                Element::LinearCombination { left, right } => {
                    format!("linear_combination({}, {})", left.0, right.0)
                }
            };
            format!("member({element}, solutions({}))", equation.0)
        }
    };
    let relation = if claim.positive {
        relation
    } else {
        format!("not {relation}")
    };
    match &claim.according_to {
        Some(context) => format!("according_to({}, {relation})", context.0),
        None => relation,
    }
}
/// Renders an existing proof/result; it cannot look up or invent scientific facts.
pub fn render(result: &Understanding) -> String {
    let mut text = format!(
        "{:?}{} in the stored symbolic interpretation.\n{}\n",
        result.verdict,
        if result.conditional {
            " (conditional)"
        } else {
            ""
        },
        result.reason
    );
    for (index, step) in result.proof.iter().enumerate() {
        let premises = step
            .premises
            .iter()
            .map(|p| match p {
                Premise::Source { fact } => format!("fact:{fact}"),
                Premise::Assumption { index } => format!("assumption:{index}"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        text.push_str(&format!(
            "{}: {}\n  rule={} from [{}]\n",
            index + 1,
            display_claim(&step.conclusion),
            step.rule,
            premises
        ));
    }
    for fact in &result.sources {
        text.push_str(&format!(
            "fact:{} → {} (passage {}, bytes {}..{})\n",
            fact.id, fact.source.url, fact.source.passage, fact.source.start, fact.source.end
        ));
    }
    if result.display_truncated {
        text.push_str("Display limited to 16 proof entries.\n");
    }
    text
}

pub fn display_query(query: &Query) -> String {
    match query {
        Query::Describe { entity } => format!("What is {}?", entity.0),
        Query::Verify { claim } => format!("Check whether {}.", display_claim(claim)),
        Query::Conditional {
            assumptions,
            conclusion,
        } => format!(
            "Assuming [{}], check whether {}.",
            assumptions
                .iter()
                .map(display_claim)
                .collect::<Vec<_>>()
                .join("; "),
            display_claim(conclusion)
        ),
        Query::Unsupported { text } => text.clone(),
    }
}
