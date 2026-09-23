//! TC-177/178: completed depth-one finite partition, with explicit refusals.
//! Atom basis {p0}; leaves {false,true,p0}; all Boolean and temporal operators;
//! closed 0 <= a <= b <= 2 plus [0,); complete words of length 1..=3.

use std::collections::BTreeMap;

use tl_oracle::population::{CaseResult, PopulationError, PopulationLedger, PopulationSummary};
use tl_oracle::{evaluate_finite, Formula, Interval, Limits, OracleError};
use tl_syntax::PropositionId;

const CLOSED_INTERVALS: usize = 6;
const INTERVALS: usize = CLOSED_INTERVALS + 1;
const LEAVES: usize = 3;
const FORMULAS: usize =
    LEAVES + (2 + 4 * INTERVALS) * LEAVES + (4 + 4 * INTERVALS) * LEAVES * LEAVES;
// Sum(length * 2^length) for lengths 1, 2, 3.
const WORD_POSITIONS: usize = 2 + 8 + 24;
const CASES: usize = FORMULAS * WORD_POSITIONS;

fn intervals() -> Vec<Interval> {
    let mut values = Vec::new();
    for end in 0..=2 {
        for start in 0..=end {
            values.push(Interval::Closed { start, end });
        }
    }
    values.push(Interval::Unbounded { start: 0 });
    values
}

fn formulas() -> Vec<Formula> {
    use Formula as F;
    let leaves = [F::False, F::True, F::Atom(PropositionId(0))];
    let mut formulas = leaves.to_vec();
    for p in &leaves {
        formulas.push(F::Not(Box::new(p.clone())));
        formulas.push(F::StrongPrevious(Box::new(p.clone())));
        for range in intervals() {
            formulas.extend([
                F::Future(range, Box::new(p.clone())),
                F::Globally(range, Box::new(p.clone())),
                F::Once(range, Box::new(p.clone())),
                F::Historically(range, Box::new(p.clone())),
            ]);
        }
    }
    for p in &leaves {
        for q in &leaves {
            formulas.extend([
                F::And(Box::new(p.clone()), Box::new(q.clone())),
                F::Or(Box::new(p.clone()), Box::new(q.clone())),
                F::Implies(Box::new(p.clone()), Box::new(q.clone())),
                F::Equivalent(Box::new(p.clone()), Box::new(q.clone())),
            ]);
            for range in intervals() {
                formulas.extend([
                    F::Until(range, Box::new(p.clone()), Box::new(q.clone())),
                    F::Release(range, Box::new(p.clone()), Box::new(q.clone())),
                    F::Since(range, Box::new(p.clone()), Box::new(q.clone())),
                    F::Triggered(range, Box::new(p.clone()), Box::new(q.clone())),
                ]);
            }
        }
    }
    formulas
}

fn words() -> Vec<Vec<BTreeMap<PropositionId, bool>>> {
    let mut words = Vec::new();
    for len in 1..=3 {
        for bits in 0..(1 << len) {
            words.push(
                (0..len)
                    .map(|index| [(PropositionId(0), (bits & (1 << index)) != 0)].into())
                    .collect(),
            );
        }
    }
    words
}

fn finite_reference(formula: &Formula, word: &[BTreeMap<PropositionId, bool>], at: usize) -> bool {
    use Formula as F;
    fn closed_bounds(interval: Interval) -> (usize, usize) {
        match interval {
            Interval::Closed { start, end } => (start, end),
            Interval::Unbounded { .. } => unreachable!("open range is preclassified"),
        }
    }
    fn scan(
        witness: &Formula,
        guard: Option<&Formula>,
        word: &[BTreeMap<PropositionId, bool>],
        at: usize,
        past: bool,
        range: Interval,
    ) -> bool {
        let (lo, hi) = closed_bounds(range);
        let mut guard_ok = true;
        for offset in 0..=hi {
            let time = if past {
                match at.checked_sub(offset) {
                    Some(time) => time,
                    None => break,
                }
            } else {
                at + offset
            };
            if offset >= lo && guard_ok && finite_reference(witness, word, time) {
                return true;
            }
            if let Some(guard) = guard {
                guard_ok &= finite_reference(guard, word, time);
                if !guard_ok {
                    break;
                }
            }
        }
        false
    }
    match formula {
        F::False => false,
        F::True => true,
        F::Atom(id) => word
            .get(at)
            .and_then(|cell| cell.get(id))
            .copied()
            .unwrap_or(false),
        F::Not(p) => !finite_reference(p, word, at),
        F::And(p, q) => finite_reference(p, word, at) && finite_reference(q, word, at),
        F::Or(p, q) => finite_reference(p, word, at) || finite_reference(q, word, at),
        F::Implies(p, q) => !finite_reference(p, word, at) || finite_reference(q, word, at),
        F::Equivalent(p, q) => finite_reference(p, word, at) == finite_reference(q, word, at),
        F::Future(r, p) => scan(p, None, word, at, false, *r),
        F::Globally(r, p) => !scan(&F::Not(p.clone()), None, word, at, false, *r),
        F::Until(r, p, q) => scan(q, Some(p), word, at, false, *r),
        F::Release(r, p, q) => !scan(
            &F::Not(q.clone()),
            Some(&F::Not(p.clone())),
            word,
            at,
            false,
            *r,
        ),
        F::Once(r, p) => scan(p, None, word, at, true, *r),
        F::Historically(r, p) => !scan(&F::Not(p.clone()), None, word, at, true, *r),
        F::Since(r, p, q) => scan(q, Some(p), word, at, true, *r),
        F::Triggered(r, p, q) => !scan(
            &F::Not(q.clone()),
            Some(&F::Not(p.clone())),
            word,
            at,
            true,
            *r,
        ),
        F::StrongPrevious(p) => at > 0 && finite_reference(p, word, at - 1),
    }
}

fn has_open_range(formula: &Formula) -> bool {
    use Formula as F;
    matches!(
        formula,
        F::Future(Interval::Unbounded { .. }, _)
            | F::Globally(Interval::Unbounded { .. }, _)
            | F::Until(Interval::Unbounded { .. }, _, _)
            | F::Release(Interval::Unbounded { .. }, _, _)
            | F::Once(Interval::Unbounded { .. }, _)
            | F::Historically(Interval::Unbounded { .. }, _)
            | F::Since(Interval::Unbounded { .. }, _, _)
            | F::Triggered(Interval::Unbounded { .. }, _, _)
    )
}

/// TC-177/178: every declared cell is reached once, compared or refused.
#[test]
fn completed_depth_one_finite_partition() {
    let formulas = formulas();
    let words = words();
    assert_eq!(formulas.len(), FORMULAS);
    assert_eq!(words.iter().map(Vec::len).sum::<usize>(), WORD_POSITIONS);
    let mut ledger = PopulationLedger::new(CASES as u64).unwrap();
    let mut case_id = 0;
    for formula in &formulas {
        for word in &words {
            for at in 0..word.len() {
                let expected = if has_open_range(formula) {
                    CaseResult::Refused
                } else {
                    CaseResult::Admitted(finite_reference(formula, word, at))
                };
                let observed = match evaluate_finite(formula, word, at, Limits::default()) {
                    Ok(value) => CaseResult::Admitted(value),
                    Err(OracleError::UnboundedFiniteInterval) => CaseResult::Refused,
                    Err(error) => panic!("unexpected oracle error: {error:?}"),
                };
                ledger
                    .record(case_id, expected, observed)
                    .unwrap_or_else(|error| {
                        panic!("{error:?}: formula={formula:?}, word={word:?}, at={at}")
                    });
                case_id += 1;
            }
        }
    }
    assert_eq!(case_id as usize, CASES);
    // Four unary operators over three leaves, and four binary operators over nine pairs.
    let open_formulas = 4 * LEAVES + 4 * LEAVES * LEAVES;
    assert_eq!(
        ledger.finish(),
        Ok(PopulationSummary {
            declared: CASES as u64,
            admitted: ((FORMULAS - open_formulas) * WORD_POSITIONS) as u64,
            refused: (open_formulas * WORD_POSITIONS) as u64,
        })
    );
}

/// TC-178/NFR-008: seeded omissions, duplicates, unknown IDs and wrong results fail.
#[test]
fn seeded_population_faults_are_detected() {
    let mut ledger = PopulationLedger::new(2).unwrap();
    assert_eq!(
        ledger.record(0, CaseResult::Admitted(true), CaseResult::Admitted(false)),
        Err(PopulationError::Mismatch { case_id: 0 })
    );
    assert_eq!(
        ledger.record(0, CaseResult::<bool>::Refused, CaseResult::Refused),
        Err(PopulationError::Duplicate { case_id: 0 })
    );
    assert_eq!(
        ledger.record(2, CaseResult::<bool>::Refused, CaseResult::Refused),
        Err(PopulationError::OutOfDomain { case_id: 2 })
    );
    assert_eq!(
        ledger.finish(),
        Err(PopulationError::Mismatch { case_id: 0 })
    );
    let mut omitted = PopulationLedger::new(2).unwrap();
    omitted
        .record(0, CaseResult::Admitted(true), CaseResult::Admitted(true))
        .unwrap();
    assert_eq!(
        omitted.finish(),
        Err(PopulationError::Incomplete {
            declared: 2,
            visited: 1,
        })
    );
    assert!(matches!(
        PopulationLedger::new(0),
        Err(PopulationError::Empty)
    ));
}
