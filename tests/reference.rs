use std::collections::BTreeMap;

use tl_oracle::{
    evaluate, evaluate_documents, evaluate_finite, evaluate_syntax, Evidence, Formula as F,
    Interval as I, Lasso, Limits, OracleError, Uncertainty, Verdict,
};
use tl_syntax::{
    InfiniteClock, InfiniteFormulaDocument, InfiniteNode, InfiniteNodeKind, LassoTraceDocument,
    NodeId, PartialValuation, PartialValue, PropositionId, SemanticProfile, TemporalInterval,
    TraceObservation, UnboundedInterval, ValuationEntry,
};

fn cell(values: &[(u32, Evidence)]) -> BTreeMap<PropositionId, Evidence> {
    values
        .iter()
        .map(|(id, value)| (PropositionId(*id), *value))
        .collect()
}

/// TC-175, FR-043-AC-1: closed finite offsets include the last cell and origin.
#[test]
fn tc_175_closed_finite_reference_fixtures() {
    let cells: Vec<BTreeMap<PropositionId, bool>> = [false, true]
        .into_iter()
        .map(|value| [(PropositionId(0), value)].into_iter().collect())
        .collect();
    let range = I::Closed { start: 1, end: 1 };
    let check =
        |formula: F, position| evaluate_finite(&formula, &cells, position, Limits::default());
    assert_eq!(check(F::Future(range, Box::new(atom(0))), 0), Ok(true));
    assert_eq!(check(F::Future(range, Box::new(atom(0))), 1), Ok(false));
    assert_eq!(check(F::Globally(range, Box::new(atom(0))), 1), Ok(false));
    assert_eq!(check(F::Once(range, Box::new(atom(0))), 1), Ok(false));
    assert_eq!(
        check(F::Historically(range, Box::new(atom(0))), 0),
        Ok(true)
    );
    assert_eq!(
        check(F::Since(range, Box::new(F::True), Box::new(atom(0))), 0),
        Ok(false)
    );
    assert_eq!(
        check(F::Triggered(range, Box::new(F::True), Box::new(atom(0))), 0),
        Ok(true)
    );
    assert_eq!(
        check(F::Until(range, Box::new(F::True), Box::new(atom(0))), 0),
        Ok(true)
    );
    assert_eq!(
        check(F::Release(range, Box::new(F::False), Box::new(atom(0))), 0),
        Ok(true)
    );
    assert_eq!(
        check(F::Future(unbounded(), Box::new(atom(0))), 0),
        Err(OracleError::UnboundedFiniteInterval)
    );
    assert_eq!(
        check(
            F::Or(
                Box::new(F::True),
                Box::new(F::Future(unbounded(), Box::new(atom(0)))),
            ),
            0,
        ),
        Err(OracleError::UnboundedFiniteInterval)
    );
}

fn atom(id: u32) -> F {
    F::Atom(PropositionId(id))
}

fn unbounded() -> I {
    I::Unbounded { start: 0 }
}

fn verdict(formula: &F, word: &Lasso, position: usize) -> Verdict {
    evaluate(formula, &[], word, position, Limits::default())
        .unwrap()
        .verdict
}

/// TC-175, FR-043-AC-1: independent truth at the first and repeated lap.
#[test]
fn tc_175_future_and_past_reference_fixtures() {
    let word = Lasso::new(
        vec![],
        vec![cell(&[(0, Evidence::True)]), cell(&[(0, Evidence::False)])],
    )
    .unwrap();
    let p = atom(0);
    assert_eq!(
        verdict(&F::Future(unbounded(), Box::new(p.clone())), &word, 0),
        Verdict::Proved
    );
    assert_eq!(
        verdict(&F::Globally(unbounded(), Box::new(p.clone())), &word, 0),
        Verdict::Refuted
    );
    assert_eq!(
        verdict(&F::Historically(unbounded(), Box::new(p.clone())), &word, 0),
        Verdict::Proved
    );
    assert_eq!(
        verdict(&F::Historically(unbounded(), Box::new(p.clone())), &word, 2),
        Verdict::Refuted
    );
    assert_eq!(
        verdict(&F::StrongPrevious(Box::new(p.clone())), &word, 0),
        Verdict::Refuted
    );
    assert_eq!(
        verdict(&F::StrongPrevious(Box::new(p)), &word, 2),
        Verdict::Refuted
    );
}

/// TC-193, FR-053-AC-1: future reads a past result after multiple loop laps.
#[test]
fn tc_193_nested_past_future_keeps_origin_state() {
    let word = Lasso::new(
        vec![],
        vec![cell(&[(0, Evidence::True)]), cell(&[(0, Evidence::False)])],
    )
    .unwrap();
    let h = F::Historically(unbounded(), Box::new(atom(0)));
    let eventually_h = F::Future(unbounded(), Box::new(h));
    assert_eq!(verdict(&eventually_h, &word, 0), Verdict::Proved);
    assert_eq!(verdict(&eventually_h, &word, 2), Verdict::Refuted);

    let since = F::Since(unbounded(), Box::new(F::True), Box::new(atom(0)));
    assert_eq!(verdict(&since, &word, 0), Verdict::Proved);
    assert_eq!(verdict(&since, &word, 1), Verdict::Proved);
    assert_eq!(verdict(&since, &word, 5), Verdict::Proved);
    let triggered = F::Triggered(
        I::Closed { start: 1, end: 1 },
        Box::new(atom(0)),
        Box::new(F::False),
    );
    assert_eq!(verdict(&triggered, &word, 0), Verdict::Proved);
}

/// TC-193, FR-053-AC-1: one loop lap moved into the prefix preserves the word.
#[test]
fn tc_193_lasso_unrolling_preserves_mixed_semantics() {
    let first = cell(&[(0, Evidence::False)]);
    let second = cell(&[(0, Evidence::True)]);
    let original = Lasso::new(vec![], vec![first.clone(), second.clone()]).unwrap();
    let unrolled = Lasso::new(vec![first.clone(), second.clone()], vec![first, second]).unwrap();
    let p = atom(0);
    let mixed = [
        F::Future(
            unbounded(),
            Box::new(F::Historically(unbounded(), Box::new(p.clone()))),
        ),
        F::Globally(
            unbounded(),
            Box::new(F::Once(unbounded(), Box::new(p.clone()))),
        ),
        F::Since(
            I::Unbounded { start: 1 },
            Box::new(F::Future(unbounded(), Box::new(p.clone()))),
            Box::new(p.clone()),
        ),
        F::Until(
            I::Closed { start: 1, end: 3 },
            Box::new(F::Once(unbounded(), Box::new(p.clone()))),
            Box::new(p),
        ),
    ];
    for formula in &mixed {
        for position in 0..10 {
            assert_eq!(
                verdict(formula, &original, position),
                verdict(formula, &unrolled, position),
                "formula={formula:?}, position={position}"
            );
        }
    }
}

/// TC-193, FR-053-AC-1: graph lowering uses syntax's public profile and interval.
#[test]
fn tc_193_syntax_graph_adapter_is_exhaustive() {
    let doc = InfiniteFormulaDocument::new(
        SemanticProfile::InfiniteTraceV1,
        InfiniteClock::EventPosition,
        NodeId(1),
        vec![
            InfiniteNode::new(InfiniteNodeKind::Proposition {
                proposition: PropositionId(0),
            }),
            InfiniteNode::new(InfiniteNodeKind::Future {
                interval: TemporalInterval::Unbounded(UnboundedInterval::new(0)),
                operand: NodeId(0),
            }),
        ],
    )
    .unwrap();
    let word = Lasso::new(
        vec![],
        vec![cell(&[(0, Evidence::False)]), cell(&[(0, Evidence::True)])],
    )
    .unwrap();
    let result =
        evaluate_syntax(doc.formula(), NodeId(1), &[], &word, 0, Limits::default()).unwrap();
    assert_eq!(result.verdict, Verdict::Proved);
    assert_eq!(
        evaluate_syntax(doc.formula(), NodeId(7), &[], &word, 0, Limits::default()),
        Err(OracleError::GraphRootAbsent)
    );
}

/// TC-193, FR-053-AC-1: syntax trace adapter preserves four-valued evidence.
#[test]
fn tc_193_validated_document_pair() {
    let formula = InfiniteFormulaDocument::new(
        SemanticProfile::InfiniteTraceV1,
        InfiniteClock::EventPosition,
        NodeId(0),
        vec![InfiniteNode::new(InfiniteNodeKind::Proposition {
            proposition: PropositionId(0),
        })],
    )
    .unwrap();
    let map = "map-1".to_owned();
    let valuation = PartialValuation::new(
        map.clone(),
        &[PropositionId(0)],
        vec![ValuationEntry {
            proposition: PropositionId(0),
            value: PartialValue::Conflicting,
        }],
    )
    .unwrap();
    let trace = LassoTraceDocument::new(
        SemanticProfile::InfiniteTraceV1,
        InfiniteClock::EventPosition,
        map,
        vec![PropositionId(0)],
        vec![],
        vec![TraceObservation {
            position: 0,
            valuation,
        }],
    )
    .unwrap();
    let result =
        evaluate_documents(&formula, &trace, NodeId(0), &[], 4, Limits::default()).unwrap();
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert_eq!(result.uncertainty, Uncertainty::Conflicting);
    assert_eq!(result.fair_completions, 2);
}

/// TC-175, FR-043-AC-1: common completions keep repeated references correlated.
#[test]
fn tc_175_partial_completions_and_fairness() {
    let word = Lasso::new(vec![], vec![cell(&[(0, Evidence::Missing)])]).unwrap();
    let contradiction = F::And(Box::new(atom(0)), Box::new(F::Not(Box::new(atom(0)))));
    let result = evaluate(&contradiction, &[], &word, 8, Limits::default()).unwrap();
    assert_eq!(result.verdict, Verdict::Refuted);
    assert_eq!(result.fair_completions, 2);
    assert_eq!(result.uncertainty, Uncertainty::Missing);

    let result = evaluate(&atom(0), &[atom(0)], &word, 3, Limits::default()).unwrap();
    assert_eq!(result.verdict, Verdict::Proved);
    assert_eq!(result.fair_completions, 1);
    let conflict = Lasso::new(vec![], vec![cell(&[(0, Evidence::Conflicting)])]).unwrap();
    let result = evaluate(&atom(0), &[], &conflict, 0, Limits::default()).unwrap();
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert_eq!(result.uncertainty, Uncertainty::Conflicting);

    let extra = Lasso::new(
        vec![],
        vec![cell(&[(0, Evidence::True), (1, Evidence::Missing)])],
    )
    .unwrap();
    let result = evaluate(&atom(0), &[], &extra, 0, Limits::default()).unwrap();
    assert_eq!(result.verdict, Verdict::Proved);
    assert_eq!(result.fair_completions, 2);
}

/// TC-194, FR-053-AC-2: no fair trace or exhausted enumeration earns a verdict.
#[test]
fn tc_194_refusals_and_limits_do_not_prove() {
    let word = Lasso::new(vec![], vec![cell(&[(0, Evidence::False)])]).unwrap();
    assert_eq!(
        evaluate(&F::True, &[atom(0)], &word, 0, Limits::default()),
        Err(OracleError::EmptyFairAdmission)
    );
    let unknown = Lasso::new(vec![], vec![cell(&[(0, Evidence::Missing)])]).unwrap();
    assert_eq!(
        evaluate(
            &atom(0),
            &[],
            &unknown,
            0,
            Limits {
                max_completions: 1,
                ..Limits::default()
            }
        ),
        Err(OracleError::ResourceIncomplete)
    );
    let impossible = Lasso::new(vec![], vec![cell(&[(0, Evidence::Impossible)])]).unwrap();
    assert_eq!(
        evaluate(&atom(0), &[], &impossible, 0, Limits::default()),
        Err(OracleError::InconsistentInput)
    );
}

/// TC-194, FR-053-AC-2: shared syntax DAG expansion stays resource bounded.
#[test]
fn tc_194_graph_expansion_budget() {
    let mut nodes = vec![InfiniteNode::new(InfiniteNodeKind::True)];
    for index in 1..19 {
        let previous = NodeId(index - 1);
        nodes.push(InfiniteNode::new(InfiniteNodeKind::And {
            left: previous,
            right: previous,
        }));
    }
    let doc = InfiniteFormulaDocument::new(
        SemanticProfile::InfiniteTraceV1,
        InfiniteClock::EventPosition,
        NodeId(18),
        nodes,
    )
    .unwrap();
    assert_eq!(
        tl_oracle::from_syntax(doc.formula(), NodeId(18)),
        Err(OracleError::ResourceIncomplete)
    );
}

/// TC-176, FR-043-AC-2: an independently seeded wrong answer is detected.
#[test]
fn tc_176_wrong_truth_clause_is_detected() {
    let word = Lasso::new(
        vec![],
        vec![cell(&[(0, Evidence::False)]), cell(&[(0, Evidence::True)])],
    )
    .unwrap();
    let claim = F::Future(unbounded(), Box::new(atom(0)));
    let actual = verdict(&claim, &word, 0);
    let seeded_wrong_clause = Verdict::Refuted;
    assert_eq!(actual, Verdict::Proved);
    assert_ne!(actual, seeded_wrong_clause);
}
