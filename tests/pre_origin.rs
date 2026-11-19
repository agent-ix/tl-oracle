//! FSpec FR-092 "Positions before the origin": on a lasso, whose position 0 is
//! the origin, a closed past interval reads the pre-origin extension (atoms
//! false, constants unchanged, operators evaluated there) and never skips a
//! pre-origin offset; the unbounded and `[a,*]` past operators range over the
//! positions 0 through `t - a` only. Both lasso evaluators are held to it.

use std::collections::BTreeMap;

use tl_oracle::fixpoint::decide_complete;
use tl_oracle::{evaluate, Evidence, Formula as F, Interval as I, Lasso, Limits, Verdict};
use tl_syntax::PropositionId;

fn p() -> F {
    F::Atom(PropositionId(0))
}
fn b(f: F) -> Box<F> {
    Box::new(f)
}
fn not(f: F) -> F {
    F::Not(b(f))
}
fn closed(start: usize, end: usize) -> I {
    I::Closed { start, end }
}
fn open(start: usize) -> I {
    I::Unbounded { start }
}

/// `p` as a bit string over prefix and loop, the loop starting at `loop_entry`.
fn cells(bits: &str) -> Vec<BTreeMap<PropositionId, bool>> {
    bits.chars()
        .map(|c| [(PropositionId(0), c == '1')].into_iter().collect())
        .collect()
}

/// The truth of `f` at `at` from the scanning evaluator and the fixpoint
/// procedure, which must agree.
fn truth(f: &F, bits: &str, loop_entry: usize, at: usize) -> bool {
    let word = cells(bits);
    let evidence = |cell: &BTreeMap<PropositionId, bool>| {
        cell.iter()
            .map(|(id, v)| (*id, if *v { Evidence::True } else { Evidence::False }))
            .collect()
    };
    let lasso = Lasso::new(
        word[..loop_entry].iter().map(evidence).collect(),
        word[loop_entry..].iter().map(evidence).collect(),
    )
    .unwrap();
    let scanned = evaluate(f, &[], &lasso, at, Limits::default()).unwrap();
    let fixed = decide_complete(f, &word, loop_entry, at, Limits::default()).unwrap();
    let scanned = match scanned.verdict {
        Verdict::Proved => true,
        Verdict::Refuted => false,
        Verdict::Inconclusive => panic!("a complete lasso settles"),
    };
    assert_eq!(scanned, fixed, "{f:?} on {bits} at {at}");
    fixed
}

/// FSpec FR-092-AC-9: where `p` holds at every position, `once[1,1] p` at 0 is
/// false, `once[1,1] not p` at 0 is true, `historically[0,2] p` at 1 is false,
/// `historically[0,2] true` at 1 is true and `true since[1,1] not p` at 0 is
/// true; the unbounded `historically p` at 1 is true and `once[2,*] not p` at
/// 1 is false.
#[test]
fn fr_092_ac_9_cases_hold_on_a_lasso() {
    let all_p = "1";
    assert!(!truth(&F::Once(closed(1, 1), b(p())), all_p, 0, 0));
    assert!(truth(&F::Once(closed(1, 1), b(not(p()))), all_p, 0, 0));
    assert!(truth(&F::StrongPrevious(b(not(p()))), all_p, 0, 0));
    assert!(!truth(&F::Historically(closed(0, 2), b(p())), all_p, 0, 1));
    assert!(truth(&F::Historically(closed(0, 2), b(p())), all_p, 0, 2));
    assert!(truth(
        &F::Historically(closed(0, 2), b(F::True)),
        all_p,
        0,
        1
    ));
    assert!(truth(
        &F::Since(closed(1, 1), b(F::True), b(not(p()))),
        all_p,
        0,
        0
    ));
    assert!(truth(&F::Historically(open(0), b(p())), all_p, 0, 1));
    assert!(!truth(&F::Once(open(2), b(not(p()))), all_p, 0, 1));
}

/// FSpec FR-092: `p triggered[a,b] q` is the dual of `not p since[a,b] not q`
/// under the same reading, so a pre-origin `not q` witness falsifies it.
#[test]
fn triggered_is_the_dual_before_the_origin() {
    let t = F::Triggered(closed(1, 2), b(not(p())), b(p()));
    // `p since[1,2] not p` finds `not p` at -1.
    assert!(!truth(&t, "1", 0, 0));
    assert!(!truth(&t, "1", 0, 1));
    // From position 2 on, offsets 1 and 2 read `p` at existing positions.
    assert!(truth(&t, "1", 0, 2));
}

/// FSpec FR-092: operators evaluate at pre-origin positions from the extension,
/// recursively. A future operator there reads forward into the trace; an
/// unbounded past operator there reads no position at all.
#[test]
fn nested_operators_evaluate_at_pre_origin_positions() {
    // `once[1,1] eventually[0,1] p` at 0: `eventually[0,1] p` at -1 sees p at 0.
    let f = F::Once(closed(1, 1), b(F::Future(closed(0, 1), b(p()))));
    assert!(truth(&f, "10", 1, 0));
    assert!(!truth(&f, "01", 1, 0));
    // `once[2,2] always[0,0] not p` at 0 and 1 reads `not p` before the origin.
    let g = F::Once(closed(2, 2), b(F::Globally(closed(0, 0), b(not(p())))));
    assert!(truth(&g, "1", 0, 0));
    assert!(truth(&g, "1", 0, 1));
    assert!(!truth(&g, "1", 0, 2));
    // `once[1,1] historically p` at 0: the unbounded window at -1 is empty, so
    // `historically p` is true there.
    let h = F::Once(closed(1, 1), b(F::Historically(open(0), b(p()))));
    assert!(truth(&h, "0", 0, 0));
    // `once[1,1] once[0,*] not p` at 0: the empty window at -1 finds nothing.
    let o = F::Once(closed(1, 1), b(F::Once(open(0), b(not(p())))));
    assert!(!truth(&o, "1", 0, 0));
    // `once[1,1] (true since not p)` at 0: unbounded since is false at -1.
    let s = F::Once(closed(1, 1), b(F::Since(open(0), b(F::True), b(not(p())))));
    assert!(!truth(&s, "1", 0, 0));
    // `historically[1,1] (false triggered p)` at 0: unbounded triggered is
    // true at -1.
    let tr = F::Historically(closed(1, 1), b(F::Triggered(open(0), b(F::False), b(p()))));
    assert!(truth(&tr, "0", 0, 0));
}

/// FSpec FR-092: `[a,*]` past operators with a nonzero lower bound read only
/// positions 0 through `t - a`, while the closed `[a,b]` reads before 0.
#[test]
fn lower_bounded_unbounded_past_stops_at_the_origin() {
    // `once[1,*] not p` on all-p: no position 0..t-1 has `not p`.
    for at in 0..4 {
        assert!(!truth(&F::Once(open(1), b(not(p()))), "1", 0, at));
    }
    // `historically[1,*] p` on all-p holds everywhere, vacuously at 0.
    for at in 0..4 {
        assert!(truth(&F::Historically(open(1), b(p())), "1", 0, at));
    }
    // `historically[1,3] p` on all-p fails until offset 3 stays inside.
    for at in 0..3 {
        assert!(!truth(&F::Historically(closed(1, 3), b(p())), "1", 0, at));
    }
    assert!(truth(&F::Historically(closed(1, 3), b(p())), "1", 0, 3));
}
