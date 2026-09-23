//! TC-193/194: a distinct, counted V11 lasso/fairness/partial partition.

use std::collections::BTreeSet;

use tl_oracle::population::{CaseResult, PopulationError, PopulationLedger, PopulationSummary};
use tl_oracle::v11::{
    EvidenceClass, Fairness, V11Population, ADMITTED_CASES, ANCHORS, COMPLETE_WORD_COUNT,
    DECLARED_CASES, EMPTY_FAIR_CASES, EMPTY_FAIR_WORD_MODES, FORMULA_COUNT, MIXED_WORD_COUNT,
    SINGLE_UNKNOWN_WORD_COUNT, WORD_COUNT,
};
use tl_oracle::{evaluate, from_syntax, Formula, Lasso, Limits, OracleError, Verdict};
use tl_syntax::{FairnessPremisesDocument, InfiniteClock, NodeId};

fn selected<'a>(mode: Fairness, p: &'a Formula, q: &'a Formula) -> &'a [Formula] {
    match mode {
        Fairness::None => &[],
        Fairness::P => std::slice::from_ref(p),
        Fairness::Q => std::slice::from_ref(q),
    }
}

/// Tracing: TC-193, FR-053-AC-1. Every declared case is visited once and its fair
/// admission count agrees with an independent Boolean completion calculation.
#[test]
fn tc_193_v11_distinct_lasso_population_has_complete_oracle_census() {
    let population = V11Population::new();
    assert_eq!(population.formulas().len(), FORMULA_COUNT);
    assert_eq!(population.words().len(), WORD_COUNT);
    assert_eq!(DECLARED_CASES, 133_920);
    let mut classes = [0_usize; 4];
    let mut distinct = BTreeSet::new();
    for word in population.words() {
        let class = match word.evidence_class {
            EvidenceClass::Complete => 0,
            EvidenceClass::Missing => 1,
            EvidenceClass::Conflicting => 2,
            EvidenceClass::Mixed => 3,
        };
        classes[class] += 1;
        assert_eq!(word.index, distinct.len());
        assert!(distinct.insert(format!("{}:{:?}", word.prefix_len, word.cells())));
    }
    assert_eq!(classes, [COMPLETE_WORD_COUNT, 68, 68, MIXED_WORD_COUNT]);
    assert_eq!(classes[1] + classes[2], SINGLE_UNKNOWN_WORD_COUNT);
    let mut ledger = PopulationLedger::new(DECLARED_CASES).unwrap();
    let expected_empty_word_modes = population
        .words()
        .iter()
        .flat_map(|word| Fairness::ALL.map(|fairness| word.expected_fair_completions(fairness)))
        .filter(|count| *count == 0)
        .count();
    assert_eq!(expected_empty_word_modes, EMPTY_FAIR_WORD_MODES);
    assert_eq!(EMPTY_FAIR_CASES, 25_200);
    assert_eq!(ADMITTED_CASES, 108_720);
    let mut admitted = 0_u64;
    let mut empty_fair = 0_u64;
    let mut expected_admitted = 0_u64;
    let mut expected_empty_fair = 0_u64;
    let mut verdicts = [0_u64; 3];
    for (formula_index, formula) in population.formulas().iter().enumerate() {
        let graph_identity = formula.graph.content_identity().unwrap();
        for fairness in [Fairness::P, Fairness::Q] {
            FairnessPremisesDocument::new(
                &formula.graph,
                graph_identity.clone(),
                InfiniteClock::EventPosition,
                fairness.roots().to_vec(),
            )
            .unwrap_or_else(|error| panic!("{}: fairness {fairness:?}: {error:?}", formula.name));
        }
        let syntax = formula.graph.formula();
        let claim = from_syntax(syntax, syntax.root()).unwrap();
        let p = from_syntax(syntax, NodeId(0)).unwrap();
        let q = from_syntax(syntax, NodeId(1)).unwrap();
        for (word_index, word) in population.words().iter().enumerate() {
            let lasso = Lasso::from(&word.trace);
            for (fairness_index, fairness) in Fairness::ALL.into_iter().enumerate() {
                let expected_count = word.expected_fair_completions(fairness);
                let premises = selected(fairness, &p, &q);
                for (anchor_index, anchor) in ANCHORS.into_iter().enumerate() {
                    let case_id = population
                        .case_id(formula_index, word_index, fairness_index, anchor_index)
                        .unwrap();
                    let actual = evaluate(
                        &claim,
                        premises,
                        &lasso,
                        usize::try_from(anchor).unwrap(),
                        Limits::default(),
                    );
                    let expected = if expected_count == 0 {
                        expected_empty_fair += 1;
                        CaseResult::Refused
                    } else {
                        expected_admitted += 1;
                        CaseResult::Admitted(expected_count)
                    };
                    let observed = match actual {
                        Ok(outcome) => {
                            let index = match outcome.verdict {
                                Verdict::Proved => 0,
                                Verdict::Refuted => 1,
                                Verdict::Inconclusive => 2,
                            };
                            verdicts[index] += 1;
                            admitted += 1;
                            CaseResult::Admitted(u64::try_from(outcome.fair_completions).unwrap())
                        }
                        Err(OracleError::EmptyFairAdmission) => {
                            empty_fair += 1;
                            CaseResult::Refused
                        }
                        Err(error) => panic!(
                            "V11 incomplete: formula={}, word={word_index}, fairness={fairness:?}, anchor={anchor}, error={error:?}",
                            formula.name
                        ),
                    };
                    ledger.record(case_id, expected, observed).unwrap_or_else(|error| {
                        panic!(
                            "V11 mismatch {error:?}: formula={}, word={word_index}, fairness={fairness:?}, anchor={anchor}",
                            formula.name
                        )
                    });
                }
            }
        }
    }
    assert_eq!(admitted + empty_fair, DECLARED_CASES);
    assert_eq!(expected_admitted, ADMITTED_CASES);
    assert_eq!(expected_empty_fair, EMPTY_FAIR_CASES);
    assert_eq!(admitted, expected_admitted);
    assert_eq!(empty_fair, expected_empty_fair);
    assert_eq!(verdicts.into_iter().sum::<u64>(), admitted);
    assert_eq!(
        ledger.finish(),
        Ok(PopulationSummary {
            declared: DECLARED_CASES,
            admitted,
            refused: empty_fair,
        })
    );
}

/// Tracing: TC-194, FR-053-AC-2. Omissions, duplicates and a seeded false verdict
/// cannot finish or pass a declared population.
#[test]
fn tc_194_v11_faults_break_the_census_and_semantic_axes() {
    let mut ledger = PopulationLedger::new(2).unwrap();
    assert_eq!(
        ledger.record(0, CaseResult::Admitted(1), CaseResult::Admitted(2)),
        Err(PopulationError::Mismatch { case_id: 0 })
    );
    assert_eq!(
        ledger.record(0, CaseResult::Admitted(1), CaseResult::Admitted(1)),
        Err(PopulationError::Duplicate { case_id: 0 })
    );
    assert_eq!(
        ledger.record(2, CaseResult::Admitted(1), CaseResult::Admitted(1)),
        Err(PopulationError::OutOfDomain { case_id: 2 })
    );
    assert_eq!(
        ledger.finish(),
        Err(PopulationError::Mismatch { case_id: 0 })
    );
    let mut omitted = PopulationLedger::new(2).unwrap();
    omitted
        .record(0, CaseResult::Admitted(1), CaseResult::Admitted(1))
        .unwrap();
    assert_eq!(
        omitted.finish(),
        Err(PopulationError::Incomplete {
            declared: 2,
            visited: 1,
        })
    );

    let population = V11Population::new();
    let word = population
        .words()
        .iter()
        .find(|word| {
            word.prefix_len == 1
                && word.cells().len() == 2
                && word.cells()[0][0] == tl_syntax::PartialValue::False
                && word.cells()[1][0] == tl_syntax::PartialValue::True
                && word.cells()[1][1] == tl_syntax::PartialValue::False
        })
        .unwrap();
    let lasso = Lasso::from(&word.trace);
    let safety = population
        .formulas()
        .iter()
        .find(|formula| formula.name == "globally-unbounded-0")
        .unwrap();
    let syntax = safety.graph.formula();
    let claim = from_syntax(syntax, syntax.root()).unwrap();
    let p = from_syntax(syntax, NodeId(0)).unwrap();
    let q = from_syntax(syntax, NodeId(1)).unwrap();
    let actual = evaluate(&claim, &[], &lasso, 0, Limits::default()).unwrap();
    assert_eq!(actual.verdict, Verdict::Refuted);
    assert_ne!(actual.verdict, Verdict::Proved); // seeded unsound prefix proof
    let historical = population
        .formulas()
        .iter()
        .find(|formula| formula.name == "historically-unbounded-0")
        .unwrap();
    let historical_syntax = historical.graph.formula();
    let historical_claim = from_syntax(historical_syntax, historical_syntax.root()).unwrap();
    let later = evaluate(&historical_claim, &[], &lasso, 6, Limits::default()).unwrap();
    assert_eq!(later.verdict, Verdict::Refuted);
    assert_ne!(later.verdict, Verdict::Proved); // seeded one-lap reset
    let fair = evaluate(&p, std::slice::from_ref(&q), &lasso, 0, Limits::default());
    assert_eq!(fair, Err(OracleError::EmptyFairAdmission));
    assert_ne!(fair, evaluate(&p, &[], &lasso, 0, Limits::default()));
    let limited = evaluate(
        &claim,
        &[],
        &lasso,
        0,
        Limits {
            max_depth: 1,
            ..Limits::default()
        },
    );
    assert_eq!(limited, Err(OracleError::ResourceIncomplete));
}
