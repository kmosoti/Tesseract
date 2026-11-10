//! Reasoning operates on validated symbolic claims, never source prose or English.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
type SignedWitnesses = (Option<usize>, Option<usize>);
type ClaimKey = (Atom, Option<Concept>);
fn opposite(a: &Claim, b: &Claim) -> bool {
    a.atom == b.atom && a.according_to == b.according_to && a.positive != b.positive
}
fn root(parents: &mut [usize], mut index: usize) -> usize {
    let original = index;
    while parents[index] != index {
        index = parents[index];
    }
    let mut cursor = original;
    while parents[cursor] != cursor {
        let next = parents[cursor];
        parents[cursor] = index;
        cursor = next;
    }
    index
}
/// Indexed once per query. Negative aliases invalidate the entire positive
/// equivalence component containing both endpoints, including longer paths.
struct AliasIndex {
    edges: BTreeMap<Concept, Vec<(Concept, usize)>>,
    component: BTreeMap<Concept, usize>,
    conflicts: BTreeMap<usize, (Concept, Concept, usize)>,
}
struct AliasView {
    paths: BTreeMap<Concept, Vec<usize>>,
    conflicts: Vec<usize>,
    depth_limited: bool,
}
impl AliasIndex {
    fn new(graph: &Interpretation) -> Self {
        let mut pairs: BTreeMap<(Concept, Concept), (Option<usize>, Option<usize>)> =
            BTreeMap::new();
        for fact in &graph.facts {
            let Atom::Relation {
                subject,
                predicate: Relation::Alias,
                object,
                scope: Scope::Unqualified,
            } = &fact.claim.atom
            else {
                continue;
            };
            if fact.claim.according_to.is_some() {
                continue;
            }
            let pair = if subject <= object {
                (subject.clone(), object.clone())
            } else {
                (object.clone(), subject.clone())
            };
            let entry = pairs.entry(pair).or_default();
            if fact.claim.positive {
                entry.0.get_or_insert(fact.id);
            } else {
                entry.1.get_or_insert(fact.id);
            }
        }
        let mut nodes = BTreeMap::new();
        for (a, b) in pairs.keys() {
            for name in [a, b] {
                let next = nodes.len();
                nodes.entry(name.clone()).or_insert(next);
            }
        }
        let mut parents: Vec<_> = (0..nodes.len()).collect();
        let mut edges: BTreeMap<Concept, Vec<(Concept, usize)>> = BTreeMap::new();
        for ((a, b), (positive, _)) in &pairs {
            if let Some(id) = positive {
                let x = root(&mut parents, nodes[a]);
                let y = root(&mut parents, nodes[b]);
                parents[x] = y;
                edges.entry(a.clone()).or_default().push((b.clone(), *id));
                edges.entry(b.clone()).or_default().push((a.clone(), *id));
            }
        }
        let component: BTreeMap<_, _> = nodes
            .into_iter()
            .map(|(name, id)| (name, root(&mut parents, id)))
            .collect();
        let mut conflicts = BTreeMap::new();
        for ((a, b), (_, negative)) in pairs {
            if let Some(id) = negative
                && component[&a] == component[&b]
            {
                conflicts.entry(component[&a]).or_insert((a, b, id));
            }
        }
        Self {
            edges,
            component,
            conflicts,
        }
    }
    fn resolve(&self, entity: &Concept) -> Result<AliasView> {
        if let Some((start, end, negative)) = self
            .component
            .get(entity)
            .and_then(|c| self.conflicts.get(c))
        {
            let mut seen = BTreeSet::from([start.clone()]);
            let mut queue = vec![(start.clone(), vec![])];
            let mut cursor = 0;
            while cursor < queue.len() {
                let (current, path) = queue[cursor].clone();
                cursor += 1;
                if &current == end {
                    let mut conflicts = path;
                    conflicts.push(*negative);
                    return Ok(AliasView {
                        paths: BTreeMap::from([(entity.clone(), vec![])]),
                        conflicts,
                        depth_limited: false,
                    });
                }
                for (next, id) in self.edges.get(&current).into_iter().flatten() {
                    if seen.insert(next.clone()) {
                        if seen.len() > 64 {
                            return Err(Error::Budget(
                                "alias conflict witness exceeds 64 concepts".into(),
                            ));
                        }
                        let mut path = path.clone();
                        path.push(*id);
                        queue.push((next.clone(), path));
                    }
                }
            }
            return Err(Error::Corrupt("alias component has no witness path".into()));
        }
        let mut paths = BTreeMap::from([(entity.clone(), vec![])]);
        let mut queue = vec![entity.clone()];
        let mut cursor = 0;
        let mut depth_limited = false;
        while cursor < queue.len() {
            let current = queue[cursor].clone();
            cursor += 1;
            let path = paths[&current].clone();
            for (next, id) in self.edges.get(&current).into_iter().flatten() {
                if paths.contains_key(next) {
                    continue;
                }
                if path.len() == 4 {
                    depth_limited = true;
                    continue;
                }
                if paths.len() >= 64 {
                    return Err(Error::Budget("alias search exceeds 64 concepts".into()));
                }
                let mut next_path = path.clone();
                next_path.push(*id);
                paths.insert(next.clone(), next_path);
                queue.push(next.clone());
            }
        }
        Ok(AliasView {
            paths,
            conflicts: vec![],
            depth_limited,
        })
    }
}
fn matched(query: &Claim, fact: &Claim, names: &AliasView) -> Option<Vec<usize>> {
    if query.according_to != fact.according_to {
        return None;
    }
    if query.atom == fact.atom {
        return Some(vec![]);
    }
    if let (
        Atom::Relation {
            subject: a,
            predicate: p,
            object: b,
            scope: s,
        },
        Atom::Relation {
            subject: x,
            predicate: q,
            object: y,
            scope: t,
        },
    ) = (&query.atom, &fact.atom)
        && p == q
        && s == t
    {
        if *p == Relation::Alias && a == y && b == x {
            return Some(vec![]);
        }
        if b == y {
            return names.paths.get(x).cloned();
        }
    }
    None
}
fn verdict(support: bool, refute: bool) -> Verdict {
    match (support, refute) {
        (true, true) => Verdict::Conflict,
        (true, false) => Verdict::Supported,
        (false, true) => Verdict::Refuted,
        _ => Verdict::Unknown,
    }
}
fn result(graph: &Interpretation, query: &Query) -> Result<Understanding> {
    Ok(Understanding {query:query.clone(),verdict:Verdict::Unknown,conditional:matches!(query,Query::Conditional {..}),proof:vec![],sources:vec![],graph_digest:graph.digest()?,corpus_digest:graph.corpus_digest.clone(),reason:"No matching interpreted proposition within the four-edge alias bound; absence is not refutation.".into(),display_truncated:false})
}
fn add_sources(
    graph: &Interpretation,
    result: &mut Understanding,
    ids: impl IntoIterator<Item = usize>,
) {
    for id in ids {
        if !result.sources.iter().any(|f| f.id == id) {
            result.sources.push(graph.facts[id].clone());
        }
    }
}
fn alias_diagnostics(graph: &Interpretation, output: &mut Understanding, names: &AliasView) {
    if !names.conflicts.is_empty() {
        output.reason.push_str(
            " Alias component has conflicting evidence; equivalence rewriting was disabled.",
        );
        add_sources(graph, output, names.conflicts.iter().copied());
    }
    if names.depth_limited {
        output
            .reason
            .push_str(" Alias traversal reached its four-edge cutoff.");
    }
}
pub(super) fn evaluate(graph: &Interpretation, query: &Query) -> Result<Understanding> {
    let mut output = result(graph, query)?;
    match query {
        Query::Unsupported { .. } => {
            output.reason =
                "Question syntax has no symbolic interpretation. No premise was admitted.".into()
        }
        Query::Describe { entity } => {
            let names = AliasIndex::new(graph).resolve(entity)?;
            let mut candidates = vec![];
            for fact in &graph.facts {
                let path = match &fact.claim.atom {
                    Atom::Relation {
                        subject,
                        predicate: Relation::Alias,
                        object,
                        ..
                    } => names.paths.get(subject).or_else(|| names.paths.get(object)),
                    Atom::Relation { subject, .. } => names.paths.get(subject),
                    _ => fact
                        .claim
                        .according_to
                        .as_ref()
                        .filter(|c| *c == entity)
                        .and_then(|c| names.paths.get(c)),
                };
                if let Some(path) = path {
                    candidates.push((fact, path.clone()));
                }
            }
            let mut signs: BTreeMap<ClaimKey, SignedWitnesses> = BTreeMap::new();
            for (index, (fact, _)) in candidates.iter().enumerate() {
                let mut atom = fact.claim.atom.clone();
                if let Atom::Relation {
                    subject,
                    predicate,
                    object,
                    ..
                } = &mut atom
                {
                    if *predicate == Relation::Alias {
                        if *subject > *object {
                            std::mem::swap(subject, object);
                        }
                    } else {
                        *subject = entity.clone();
                    }
                }
                let entry = signs
                    .entry((atom, fact.claim.according_to.clone()))
                    .or_default();
                if fact.claim.positive {
                    entry.0.get_or_insert(index);
                } else {
                    entry.1.get_or_insert(index);
                }
            }
            let conflict = signs.values().find_map(|(a, b)| a.zip(*b));
            output.verdict = if conflict.is_some() || !names.conflicts.is_empty() {
                Verdict::Conflict
            } else if candidates.is_empty() {
                Verdict::Unknown
            } else {
                Verdict::Supported
            };
            let mut selected = conflict.map_or_else(Vec::new, |(a, b)| vec![a, b]);
            for index in 0..candidates.len() {
                if selected.len() == 16 {
                    break;
                }
                if !selected.contains(&index) {
                    selected.push(index);
                }
            }
            output.display_truncated = candidates.len() > selected.len();
            for index in selected {
                let (fact, path) = &candidates[index];
                output.proof.push(ProofStep {
                    rule: if path.is_empty() {
                        "source_assertion"
                    } else {
                        "alias_lookup"
                    }
                    .into(),
                    conclusion: fact.claim.clone(),
                    premises: path
                        .iter()
                        .chain(std::iter::once(&fact.id))
                        .map(|id| Premise::Source { fact: *id })
                        .collect(),
                });
                add_sources(graph, &mut output, path.iter().copied().chain([fact.id]));
            }
            if !output.proof.is_empty() {
                output.reason =
                    "Partial source-grounded concept model; only parsed claims are shown.".into();
            }
            alias_diagnostics(graph, &mut output, &names);
        }
        Query::Verify { claim } => {
            let names = match &claim.atom {
                Atom::Relation { subject, .. } => AliasIndex::new(graph).resolve(subject)?,
                _ => AliasView {
                    paths: BTreeMap::new(),
                    conflicts: vec![],
                    depth_limited: false,
                },
            };
            let mut candidates = vec![];
            for fact in &graph.facts {
                if let Some(path) = matched(claim, &fact.claim, &names) {
                    candidates.push((fact, path));
                }
            }
            let supported = candidates
                .iter()
                .any(|(f, _)| f.claim.positive == claim.positive);
            let refuted = candidates
                .iter()
                .any(|(f, _)| f.claim.positive != claim.positive);
            output.verdict = verdict(supported, refuted);
            let mut selected = vec![];
            for positive in [true, false] {
                if let Some(index) = candidates
                    .iter()
                    .position(|(f, _)| f.claim.positive == positive)
                {
                    selected.push(index);
                }
            }
            for index in 0..candidates.len() {
                if selected.len() == 16 {
                    break;
                }
                if !selected.contains(&index) {
                    selected.push(index);
                }
            }
            output.display_truncated = candidates.len() > selected.len();
            for index in selected {
                let (fact, path) = &candidates[index];
                let rule = if !path.is_empty() {
                    "alias_rewrite"
                } else if fact.claim.atom != claim.atom {
                    "alias_symmetry"
                } else {
                    "source_assertion"
                };
                output.proof.push(ProofStep {
                    rule: rule.into(),
                    conclusion: Claim {
                        atom: claim.atom.clone(),
                        positive: fact.claim.positive,
                        according_to: claim.according_to.clone(),
                    },
                    premises: path
                        .iter()
                        .chain(std::iter::once(&fact.id))
                        .map(|id| Premise::Source { fact: *id })
                        .collect(),
                });
                add_sources(graph, &mut output, path.iter().copied().chain([fact.id]));
            }
            if supported || refuted {
                output.reason="Verdict is relative to parsed source assertions, not certified scientific truth.".into();
            }
            alias_diagnostics(graph, &mut output, &names);
        }
        Query::Conditional {
            assumptions,
            conclusion,
        } => {
            if assumptions
                .iter()
                .any(|a| assumptions.iter().any(|b| opposite(a, b)))
                || assumptions.iter().any(|a| opposite(a, conclusion))
            {
                output.reason="Hypothetical assumptions conflict with each other or oppose the requested conclusion; inference withheld.".into();
                return Ok(output);
            }
            if let Some(index) = assumptions.iter().position(|a| a == conclusion) {
                output.verdict = Verdict::Supported;
                output.proof.push(ProofStep {
                    rule: "hypothesis".into(),
                    conclusion: conclusion.clone(),
                    premises: vec![Premise::Assumption { index }],
                });
                output.reason = "Restatement of an explicit hypothetical assumption.".into();
                return Ok(output);
            }
            let Atom::Solution {
                element: Element::LinearCombination { left, right },
                equation,
            } = &conclusion.atom
            else {
                return Ok(output);
            };
            if !conclusion.positive {
                return Ok(output);
            }
            let member = |name: &Concept| Claim {
                atom: Atom::Solution {
                    element: Element::Named { name: name.clone() },
                    equation: equation.clone(),
                },
                positive: true,
                according_to: conclusion.according_to.clone(),
            };
            let (Some(a), Some(b)) = (
                assumptions.iter().position(|c| c == &member(left)),
                assumptions.iter().position(|c| c == &member(right)),
            ) else {
                return Ok(output);
            };
            let closure = Claim {
                atom: Atom::ClosedUnderLinearCombination {
                    equation: equation.clone(),
                },
                positive: true,
                according_to: conclusion.according_to.clone(),
            };
            if graph.facts.iter().any(|f| opposite(&f.claim, &closure))
                || assumptions.iter().any(|a| opposite(a, &closure))
            {
                output.reason="Required closure premise has opposing evidence or an opposing assumption; inference withheld.".into();
                return Ok(output);
            }
            if let Some(fact) = graph.facts.iter().find(|f| f.claim == closure) {
                output.verdict = Verdict::Supported;
                output.proof.push(ProofStep {
                    rule: "linear_combination_closure".into(),
                    conclusion: conclusion.clone(),
                    premises: vec![
                        Premise::Source { fact: fact.id },
                        Premise::Assumption { index: a },
                        Premise::Assumption { index: b },
                    ],
                });
                add_sources(graph, &mut output, [fact.id]);
                output.reason="Conditional deduction from acquired closure and the two explicit solution assumptions.".into();
            }
        }
    }
    Ok(output)
}
