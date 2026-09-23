use std::collections::BTreeMap;

use tl_oracle::{
    evaluate_closed_trace_v1, evaluate_finite, evaluate_origin_complete, Formula as F,
    Interval as I, Limits, OracleError,
};
use tl_syntax::PropositionId;

fn atom() -> F {
    F::Atom(PropositionId(0))
}

fn cell(value: bool) -> BTreeMap<PropositionId, bool> {
    [(PropositionId(0), value)].into_iter().collect()
}

/// TC-175, FR-043-AC-1: the closed profile starts bounded U/R guards at the lower bound.
#[test]
fn tc_175_closed_profile_nonzero_until_offset() {
    let word = [cell(false), cell(true)];
    let range = I::Closed { start: 1, end: 1 };
    let until = F::Until(range, Box::new(atom()), Box::new(atom()));
    assert_eq!(
        evaluate_closed_trace_v1(&until, &word, 0, Limits::default()),
        Ok(true)
    );
    assert_eq!(
        evaluate_finite(&until, &word, 0, Limits::default()),
        Ok(false),
        "the original finite reference retains origin-start guard semantics"
    );
    let release = F::Release(range, Box::new(atom()), Box::new(atom()));
    assert_eq!(
        evaluate_closed_trace_v1(&release, &word, 0, Limits::default()),
        Ok(true)
    );
    assert_eq!(
        evaluate_closed_trace_v1(
            &F::Once(range, Box::new(atom())),
            &word,
            0,
            Limits::default(),
        ),
        Err(OracleError::ClosedTracePastUnsupported)
    );
}

/// TC-175, FR-043-AC-1: past operators visit pre-origin time with false atoms.
#[test]
fn tc_175_origin_complete_past_boundary_and_duality() {
    let word = [cell(true)];
    let range = I::Closed { start: 1, end: 1 };
    let check = |formula: F| evaluate_origin_complete(&formula, &word, 0, Limits::default());
    assert_eq!(check(F::Historically(range, Box::new(atom()))), Ok(false));
    assert_eq!(
        check(F::Historically(range, Box::new(F::Not(Box::new(atom()))))),
        Ok(true)
    );
    assert_eq!(check(F::Once(range, Box::new(atom()))), Ok(false));
    assert_eq!(check(F::StrongPrevious(Box::new(atom()))), Ok(false));
    assert_eq!(
        check(F::Since(range, Box::new(F::True), Box::new(atom()))),
        Ok(false)
    );
    assert_eq!(
        check(F::Triggered(range, Box::new(F::True), Box::new(atom()))),
        Ok(false)
    );
    assert_eq!(
        check(F::Triggered(
            range,
            Box::new(F::True),
            Box::new(F::Not(Box::new(atom())))
        )),
        Ok(true)
    );
    assert_eq!(
        check(F::Future(range, Box::new(atom()))),
        Err(OracleError::OriginFutureUnsupported)
    );
}

/// TC-175, FR-043-AC-1: the past guard begins at the interval lower endpoint.
#[test]
fn tc_175_origin_complete_nonzero_since_offset() {
    let word = [cell(false), cell(false), cell(true)];
    let range = I::Closed { start: 1, end: 2 };
    let since = F::Since(range, Box::new(F::Not(Box::new(atom()))), Box::new(atom()));
    assert_eq!(
        evaluate_origin_complete(&since, &word, 2, Limits::default()),
        Ok(false)
    );
    let word = [cell(false), cell(true), cell(false), cell(true)];
    assert_eq!(
        evaluate_origin_complete(&since, &word, 3, Limits::default()),
        Ok(true)
    );
}
