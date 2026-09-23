//! TC-179/193/194: recorded finite lasso domain and metamorphic laws.

use std::collections::BTreeMap;

use tl_oracle::{
    evaluate, Evidence, Formula as F, Interval as I, Lasso, Limits, OracleError, Outcome, Verdict,
};
use tl_syntax::PropositionId;

fn cell(p: Evidence, q: Evidence) -> BTreeMap<PropositionId, Evidence> {
    [(PropositionId(0), p), (PropositionId(1), q)]
        .into_iter()
        .collect()
}

fn result(formula: &F, fairness: &[F], word: &Lasso, at: usize) -> Outcome {
    evaluate(formula, fairness, word, at, Limits::default()).unwrap()
}

fn atom(id: u32) -> F {
    F::Atom(PropositionId(id))
}

fn neg(formula: F) -> F {
    F::Not(Box::new(formula))
}

fn dual(left: F, right: F, word: &Lasso, at: usize) {
    let a = result(&left, &[], word, at);
    let b = result(&right, &[], word, at);
    assert_eq!(a, b, "left={left:?}, right={right:?}, at={at}");
}

type Cell = BTreeMap<PropositionId, Evidence>;
type CompleteWord = (Lasso, Vec<Cell>, usize);

/// All 2^n complete p valuations for lengths 1..=3, each nonempty loop entry.
fn complete_words() -> Vec<CompleteWord> {
    let mut words = Vec::new();
    for len in 1..=3 {
        for bits in 0..(1 << len) {
            let cells: Vec<_> = (0..len)
                .map(|i| {
                    cell(
                        if bits & (1 << i) == 0 {
                            Evidence::False
                        } else {
                            Evidence::True
                        },
                        if i % 2 == 0 {
                            Evidence::True
                        } else {
                            Evidence::False
                        },
                    )
                })
                .collect();
            for loop_entry in 0..len {
                let word =
                    Lasso::new(cells[..loop_entry].to_vec(), cells[loop_entry..].to_vec()).unwrap();
                words.push((word, cells.clone(), loop_entry));
            }
        }
    }
    words
}

/// TC-179: Boolean/future/past dualities and lasso unrolling on 34 words.
#[test]
fn duality_and_unrolling_cover_complete_small_lassos() {
    let words = complete_words();
    assert_eq!(words.len(), 34);
    let p = atom(0);
    let q = atom(1);
    let intervals = [
        I::Closed { start: 0, end: 0 },
        I::Closed { start: 0, end: 2 },
        I::Closed { start: 1, end: 2 },
        I::Unbounded { start: 0 },
        I::Unbounded { start: 1 },
    ];
    let mut checks = 0;
    for (word, cells, loop_entry) in words {
        let loop_cells = cells[loop_entry..].to_vec();
        let unrolled = Lasso::new(cells, loop_cells).unwrap();
        for at in [0, 1, 3, 6] {
            dual(neg(neg(p.clone())), p.clone(), &word, at);
            for range in intervals {
                let pairs = [
                    (
                        F::Future(range, Box::new(p.clone())),
                        neg(F::Globally(range, Box::new(neg(p.clone())))),
                    ),
                    (
                        F::Until(range, Box::new(p.clone()), Box::new(q.clone())),
                        neg(F::Release(
                            range,
                            Box::new(neg(p.clone())),
                            Box::new(neg(q.clone())),
                        )),
                    ),
                    (
                        F::Once(range, Box::new(p.clone())),
                        neg(F::Historically(range, Box::new(neg(p.clone())))),
                    ),
                    (
                        F::Since(range, Box::new(p.clone()), Box::new(q.clone())),
                        neg(F::Triggered(
                            range,
                            Box::new(neg(p.clone())),
                            Box::new(neg(q.clone())),
                        )),
                    ),
                ];
                for (left, right) in pairs {
                    dual(left.clone(), right, &word, at);
                    assert_eq!(
                        result(&left, &[], &word, at),
                        result(&left, &[], &unrolled, at),
                        "unrolling: {left:?}, at={at}"
                    );
                    checks += 1;
                }
            }
        }
    }
    assert_eq!(checks, 34 * 4 * intervals.len() * 4);
}

/// TC-179/193: a fair-completion filter and evidence refinement only remove possibilities.
#[test]
fn fairness_and_partial_information_are_monotone() {
    let p = atom(0);
    let q = atom(1);
    let evidence = [
        Evidence::False,
        Evidence::True,
        Evidence::Missing,
        Evidence::Conflicting,
    ];
    let mut checks = 0;
    for first in evidence {
        for second in evidence {
            let word = Lasso::new(vec![], vec![cell(first, second)]).unwrap();
            let base = result(&p, &[], &word, 0);
            let fair = evaluate(&p, std::slice::from_ref(&q), &word, 0, Limits::default());
            if let Ok(fair) = fair {
                assert!(!fair.possible_true || base.possible_true);
                assert!(!fair.possible_false || base.possible_false);
                assert!(fair.fair_completions <= base.fair_completions);
            } else {
                assert_eq!(fair, Err(OracleError::EmptyFairAdmission));
            }
            if matches!(first, Evidence::Missing | Evidence::Conflicting) {
                for refinement in [Evidence::False, Evidence::True] {
                    let refined = Lasso::new(vec![], vec![cell(refinement, second)]).unwrap();
                    let settled = result(&p, &[], &refined, 0);
                    assert!(!settled.possible_true || base.possible_true);
                    assert!(!settled.possible_false || base.possible_false);
                    assert!(settled.fair_completions <= base.fair_completions);
                    checks += 1;
                }
            }
            checks += 1;
        }
    }
    assert_eq!(checks, 32);
}

/// TC-194: a concrete bad prefix cannot be repaired by a later loop or fairness.
#[test]
fn bad_prefix_refutation_and_seeded_axis_faults() {
    let p = atom(0);
    let safety = F::Globally(I::Unbounded { start: 0 }, Box::new(p.clone()));
    for loop_value in [Evidence::False, Evidence::True] {
        let word = Lasso::new(
            vec![cell(Evidence::False, Evidence::True)],
            vec![cell(loop_value, Evidence::True)],
        )
        .unwrap();
        let actual = result(&safety, &[atom(1)], &word, 0);
        assert_eq!(actual.verdict, Verdict::Refuted);
        assert_ne!(actual.verdict, Verdict::Proved); // seeded wrong prefix proof
    }
    let no_fair_word = Lasso::new(vec![], vec![cell(Evidence::True, Evidence::False)]).unwrap();
    let refusal = evaluate(&p, &[atom(1)], &no_fair_word, 0, Limits::default());
    assert_eq!(refusal, Err(OracleError::EmptyFairAdmission));
    assert_ne!(refusal, Ok(result(&p, &[], &no_fair_word, 0))); // seeded fair-filter bypass
}

/// TC-193/194: mixed nesting is checked beyond the first lap, including a
/// historically stable prefix state that an incorrect one-lap fixed point loses.
#[test]
fn mixed_past_future_survives_repeated_laps() {
    let p = atom(0);
    let past = I::Unbounded { start: 0 };
    let future = I::Unbounded { start: 1 };
    let mixed = [
        F::Future(future, Box::new(F::Historically(past, Box::new(p.clone())))),
        F::Globally(future, Box::new(F::Once(past, Box::new(p.clone())))),
        F::Since(
            past,
            Box::new(F::Future(future, Box::new(p.clone()))),
            Box::new(p.clone()),
        ),
        F::Until(
            future,
            Box::new(F::Once(past, Box::new(p.clone()))),
            Box::new(p),
        ),
    ];
    let mut checks = 0;
    for (word, cells, loop_entry) in complete_words() {
        let unrolled = Lasso::new(cells.clone(), cells[loop_entry..].to_vec()).unwrap();
        for formula in &mixed {
            for at in [0, 1, 3, 6, 9] {
                assert_eq!(
                    result(formula, &[], &word, at),
                    result(formula, &[], &unrolled, at),
                    "mixed formula={formula:?}, at={at}"
                );
                checks += 1;
            }
        }
    }
    assert_eq!(checks, 34 * mixed.len() * 5);

    let word = Lasso::new(
        vec![],
        vec![
            cell(Evidence::True, Evidence::True),
            cell(Evidence::False, Evidence::True),
        ],
    )
    .unwrap();
    let historical = F::Historically(past, Box::new(atom(0)));
    assert_eq!(result(&historical, &[], &word, 0).verdict, Verdict::Proved);
    assert_eq!(result(&historical, &[], &word, 2).verdict, Verdict::Refuted);
    assert_ne!(
        result(&historical, &[], &word, 2).verdict,
        Verdict::Proved // seeded one-lap state reset
    );
}
