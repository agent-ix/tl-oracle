//! TL-221: the successor-fixpoint procedure against hand-checked lassos and
//! against the offset-scanning evaluator in the crate root.
//!
//! The two procedures share only data types. Agreement on the full small-scope
//! population below is therefore evidence about both; a disagreement is a
//! defect in one of them.

use std::collections::BTreeMap;

use tl_oracle::fixpoint::{decide, decide_complete, Cell};
use tl_oracle::{evaluate, Evidence, Formula as F, Interval as I, Lasso, Limits, OracleError};
use tl_syntax::PropositionId;

fn p(id: u32) -> F {
    F::Atom(PropositionId(id))
}
fn b(f: F) -> Box<F> {
    Box::new(f)
}
fn cell(bits: &[(u32, bool)]) -> Cell {
    bits.iter().map(|(i, v)| (PropositionId(*i), *v)).collect()
}
/// Cells for the single proposition 0 from a bit string such as "0110".
fn word0(bits: &str) -> Vec<Cell> {
    bits.chars().map(|c| cell(&[(0, c == '1')])).collect()
}
fn closed(start: usize, end: usize) -> I {
    I::Closed { start, end }
}
fn open(start: usize) -> I {
    I::Unbounded { start }
}
fn holds(f: &F, prefix_and_loop: &str, loop_entry: usize, at: usize) -> bool {
    decide_complete(
        f,
        &word0(prefix_and_loop),
        loop_entry,
        at,
        Limits::default(),
    )
    .unwrap()
}

/// Hand-checked: every case below is worked out on paper from the semantics.
#[test]
fn hand_checked_lasso_cases() {
    // word 0 . (1 0)^w : p holds at odd positions forever.
    let w = "010";
    assert!(holds(&F::Future(open(0), b(p(0))), w, 1, 0));
    // G[0,) p fails: p is false at every even position.
    assert!(!holds(&F::Globally(open(0), b(p(0))), w, 1, 1));
    // G F p holds at 0 (p recurs), F G p does not.
    let gf = F::Globally(open(0), b(F::Future(open(0), b(p(0)))));
    let fg = F::Future(open(0), b(F::Globally(open(0), b(p(0)))));
    assert!(holds(&gf, w, 1, 0));
    assert!(!holds(&fg, w, 1, 0));
    // F[2,2] p at 0: position 2 is the loop-entry+1 = 0-valued cell.
    assert!(!holds(&F::Future(closed(2, 2), b(p(0))), w, 1, 0));
    assert!(holds(&F::Future(closed(3, 3), b(p(0))), w, 1, 0));
    // p U[0,) q with q only reached through the loop: word (1)(1)(0)(q at 3).
    let u = F::Until(open(0), b(p(0)), b(F::Not(b(p(0)))));
    assert!(holds(&u, "110", 2, 0));
    assert!(!holds(&u, "11", 1, 0)); // p forever, never not-p
                                     // Release: false R[0,) p is G p.
    assert!(holds(
        &F::Release(open(0), b(F::False), b(p(0))),
        "11",
        1,
        0
    ));
    assert!(!holds(
        &F::Release(open(0), b(F::False), b(p(0))),
        "101",
        1,
        0
    ));
    // Lower bound shifts without wrapping the guard: p U[2,) !p on "1110".
    let shifted = F::Until(open(2), b(p(0)), b(F::Not(b(p(0)))));
    assert!(holds(&shifted, "1110", 3, 0));
    // Past: O[0,) p at position 5 of 0.(10)^w sees the p at 1.
    assert!(holds(&F::Once(open(0), b(p(0))), "010", 1, 5));
    assert!(!holds(&F::Once(open(0), b(p(0))), "000", 1, 5));
    // Y p is false at the origin and looks one step back elsewhere.
    assert!(!holds(&F::StrongPrevious(b(p(0))), "1", 0, 0));
    assert!(holds(&F::StrongPrevious(b(p(0))), "1", 0, 4));
    // H[0,) p holds only if p held at every earlier point, including the loop.
    assert!(holds(&F::Historically(open(0), b(p(0))), "1", 0, 9));
    assert!(!holds(&F::Historically(open(0), b(p(0))), "01", 1, 9));
    // Since with a guard that breaks: (p S q).
    assert!(holds(
        &F::Since(open(0), b(p(0)), b(F::Not(b(p(0))))),
        "011",
        1,
        7
    ));
    // Since with a false guard needs q at the current point.
    assert!(!holds(
        &F::Since(open(0), b(F::False), b(p(0))),
        "011",
        1,
        0
    ));
}

/// A formula whose past stabilizes only after several loop periods, nested
/// under a future operator.
#[test]
fn nested_past_under_future_needs_unrolling() {
    // Y^3 p ... eventually: F[0,) H[3,3]... use O[3,3] p inside G.
    let inner = F::Once(closed(3, 3), b(p(0)));
    let f = F::Globally(
        open(0),
        b(F::Implies(b(F::Not(b(inner.clone()))), b(F::False))),
    );
    // inner must hold at every position for the implication to be true.
    // word: p forever, so O[3,3] p holds exactly from t = 3 on.
    assert!(!holds(&f, "111", 0, 0)); // fails at t < 3: nothing three back
    assert!(holds(&F::Globally(open(3), b(inner)), "111", 0, 0));
    // word: loop (1 0 0), O[3,3] p(t) = p(t-3) = p(t) for t >= 3 (period 3).
    let same = F::Globally(open(3), b(F::Equivalent(b(inner_p()), b(p(0)))));
    assert!(holds(&same, "100", 0, 0));
}

fn inner_p() -> F {
    F::Once(closed(3, 3), b(p(0)))
}

#[test]
fn inverted_and_oversized_intervals_refuse() {
    let l = Limits::default();
    let w = word0("01");
    assert_eq!(
        decide_complete(&F::Future(closed(3, 1), b(p(0))), &w, 0, 0, l),
        Err(OracleError::InvertedInterval)
    );
    let small = Limits { max_offset: 2, ..l };
    assert_eq!(
        decide_complete(&F::Future(closed(0, 3), b(p(0))), &w, 0, 0, small),
        Err(OracleError::ResourceIncomplete)
    );
    assert_eq!(
        decide_complete(&p(0), &[], 0, 0, l),
        Err(OracleError::EmptyLoop)
    );
}

fn intervals() -> Vec<I> {
    vec![
        closed(0, 0),
        closed(0, 2),
        closed(1, 2),
        closed(2, 3),
        open(0),
        open(2),
    ]
}

fn level_one(leaves: &[F]) -> Vec<F> {
    let mut out = Vec::new();
    for x in leaves {
        out.push(F::Not(b(x.clone())));
        out.push(F::StrongPrevious(b(x.clone())));
        for r in intervals() {
            out.push(F::Future(r, b(x.clone())));
            out.push(F::Globally(r, b(x.clone())));
            out.push(F::Once(r, b(x.clone())));
            out.push(F::Historically(r, b(x.clone())));
        }
        for y in leaves {
            out.push(F::And(b(x.clone()), b(y.clone())));
            out.push(F::Or(b(x.clone()), b(y.clone())));
            out.push(F::Implies(b(x.clone()), b(y.clone())));
            out.push(F::Equivalent(b(x.clone()), b(y.clone())));
            for r in intervals() {
                out.push(F::Until(r, b(x.clone()), b(y.clone())));
                out.push(F::Release(r, b(x.clone()), b(y.clone())));
                out.push(F::Since(r, b(x.clone()), b(y.clone())));
                out.push(F::Triggered(r, b(x.clone()), b(y.clone())));
            }
        }
    }
    out
}

/// Every complete lasso over `atoms` with prefix 0..=2 and loop 1..=3 cells.
fn lassos(atoms: u32) -> Vec<(Vec<Cell>, usize)> {
    let mut out = Vec::new();
    let states = 1u32 << atoms;
    for prefix in 0..=2usize {
        for lp in 1..=3usize {
            let total = prefix + lp;
            for code in 0..states.pow(total as u32) {
                let mut rest = code;
                let cells = (0..total)
                    .map(|_| {
                        let s = rest % states;
                        rest /= states;
                        (0..atoms)
                            .map(|a| (PropositionId(a), s >> a & 1 == 1))
                            .collect()
                    })
                    .collect();
                out.push((cells, prefix));
            }
        }
    }
    out
}

fn as_lasso(cells: &[Cell], loop_entry: usize) -> Lasso {
    let to_evidence = |c: &Cell| -> BTreeMap<PropositionId, Evidence> {
        c.iter()
            .map(|(k, v)| (*k, if *v { Evidence::True } else { Evidence::False }))
            .collect()
    };
    Lasso::new(
        cells[..loop_entry].iter().map(to_evidence).collect(),
        cells[loop_entry..].iter().map(to_evidence).collect(),
    )
    .unwrap()
}

/// Population: the fixpoint procedure and the scanning evaluator agree on every
/// formula, lasso and position of the declared small scope.
#[test]
fn agrees_with_scanning_evaluator_on_the_small_scope() {
    let limits = Limits::default();
    let leaves = [F::False, F::True, p(0)];
    let formulas = level_one(&leaves);
    let words = lassos(1);
    let mut cases = 0u64;
    for f in &formulas {
        for (cells, entry) in &words {
            let lasso = as_lasso(cells, *entry);
            for at in 0..6 {
                let reference = evaluate(f, &[], &lasso, at, limits).unwrap();
                let fixpoint = decide_complete(f, cells, *entry, at, limits).unwrap();
                assert_eq!(
                    fixpoint, reference.possible_true,
                    "{f:?} on {cells:?} entry {entry} at {at}"
                );
                cases += 1;
            }
        }
    }
    // 3 leaves => 3*(2+4*6) + 9*(4+4*6) formulas; 1,2,3 loops x 0..=2 prefixes.
    assert_eq!(formulas.len(), 3 * (2 + 4 * 6) + 9 * (4 + 4 * 6));
    // sum over prefix 0..=2 and loop 1..=3 of 2^(prefix + loop)
    assert_eq!(words.len(), 14 + 28 + 56);
    assert_eq!(cases, (formulas.len() * words.len() * 6) as u64);
}

/// Two-level nesting over two propositions, sampled by a fixed stride.
#[test]
fn agrees_with_scanning_evaluator_on_nested_formulas() {
    let limits = Limits::default();
    let seeds = [F::True, p(0), p(1), F::Not(b(p(1)))];
    let pool: Vec<F> = level_one(&seeds).into_iter().step_by(23).collect();
    let mut formulas = Vec::new();
    for x in pool.iter().chain(seeds.iter()) {
        for r in [closed(0, 1), closed(1, 2), open(0), open(1)] {
            formulas.push(F::Future(r, b(x.clone())));
            formulas.push(F::Globally(r, b(x.clone())));
            formulas.push(F::Once(r, b(x.clone())));
            formulas.push(F::Historically(r, b(x.clone())));
            for y in pool.iter().step_by(5) {
                formulas.push(F::Until(r, b(x.clone()), b(y.clone())));
                formulas.push(F::Release(r, b(x.clone()), b(y.clone())));
                formulas.push(F::Since(r, b(x.clone()), b(y.clone())));
                formulas.push(F::Triggered(r, b(x.clone()), b(y.clone())));
            }
        }
    }
    let words: Vec<_> = lassos(2).into_iter().step_by(17).collect();
    assert!(!formulas.is_empty() && !words.is_empty());
    for f in &formulas {
        for (cells, entry) in &words {
            let lasso = as_lasso(cells, *entry);
            for at in [0usize, 1, 3, 7] {
                let reference = evaluate(f, &[], &lasso, at, limits).unwrap();
                let fixpoint = decide_complete(f, cells, *entry, at, limits).unwrap();
                assert_eq!(
                    fixpoint, reference.possible_true,
                    "{f:?} {cells:?} {entry} {at}"
                );
            }
        }
    }
}

/// Partial evidence and fairness: the whole outcome, not just the verdict.
#[test]
fn partial_evidence_and_fairness_outcomes_agree() {
    use Evidence::{Conflicting as C, False as Fa, Missing as M, True as T};
    let limits = Limits::default();
    let ev = |v: &[(u32, Evidence)]| -> BTreeMap<PropositionId, Evidence> {
        v.iter().map(|(i, e)| (PropositionId(*i), *e)).collect()
    };
    let lassos = [
        Lasso::new(vec![], vec![ev(&[(0, M), (1, Fa)])]).unwrap(),
        Lasso::new(vec![], vec![ev(&[(0, C), (1, T)])]).unwrap(),
        Lasso::new(
            vec![ev(&[(0, M), (1, T)])],
            vec![ev(&[(0, T), (1, C)]), ev(&[(0, Fa)])],
        )
        .unwrap(),
        Lasso::new(
            vec![ev(&[(0, Fa), (1, Fa)])],
            vec![ev(&[(0, M), (1, T)]), ev(&[(0, Fa), (1, M)])],
        )
        .unwrap(),
    ];
    let claims = level_one(&[p(0), p(1)]);
    let fairness_sets: [Vec<F>; 5] = [
        vec![],
        vec![p(0)],
        vec![p(0), F::Not(b(p(1)))],
        // Past premises are unstable at the first loop positions.
        vec![F::Once(closed(0, 2), b(p(1)))],
        vec![
            F::Historically(open(0), b(p(0))),
            F::Future(open(0), b(p(1))),
        ],
    ];
    let mut compared = 0u64;
    for claim in claims.iter().step_by(3) {
        for lasso in &lassos {
            for fairness in &fairness_sets {
                for at in 0..4 {
                    let scan = evaluate(claim, fairness, lasso, at, limits);
                    let fix = decide(claim, fairness, lasso, at, limits);
                    assert_eq!(fix, scan, "{claim:?} fairness={fairness:?} at {at}");
                    compared += 1;
                }
            }
        }
    }
    assert!(compared > 1000);
}

/// A past premise that holds only at the first loop positions is not visited
/// infinitely often: p1 is true only at the origin, so `O[0,2] p1` holds at
/// positions 0, 1 and 2 and never again.
#[test]
fn a_past_premise_must_hold_after_it_stabilizes() {
    use Evidence::{False as Fa, True as T};
    let ev = |v: &[(u32, Evidence)]| -> BTreeMap<PropositionId, Evidence> {
        v.iter().map(|(i, e)| (PropositionId(*i), *e)).collect()
    };
    let lasso = Lasso::new(vec![ev(&[(0, T), (1, T)])], vec![ev(&[(0, T), (1, Fa)])]).unwrap();
    let premise = [F::Once(closed(0, 2), b(p(1)))];
    let l = Limits::default();
    assert_eq!(
        decide(&p(0), &premise, &lasso, 0, l),
        Err(OracleError::EmptyFairAdmission)
    );
    assert_eq!(
        evaluate(&p(0), &premise, &lasso, 0, l),
        Err(OracleError::EmptyFairAdmission)
    );
}
