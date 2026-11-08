//! A second, independent decision procedure for ultimately periodic words.
//!
//! The recursive evaluator in the crate root scans offsets window by window.
//! This module instead rewrites every temporal operator into the textbook
//! successor equations and solves them as fixpoints over the finite position
//! graph of the lasso:
//!
//! * `X U[a,b] Y` is `Y` at lower bound zero and upper bound zero, and
//!   `Y or (X and next(...[0,b-1]))` above zero; a positive lower bound only
//!   shifts one step with the guard required.
//! * Unbounded operators are the least (`U`, `F`, `S`, `O`) or greatest (`R`,
//!   `G`, `T`, `H`) solution of the same equation, iterated from the bottom or
//!   top element until it stops changing.
//! * Future operators step along the lasso successor (the last position
//!   returns to the loop entry). Past operators step to the previous time
//!   point of the unrolled word, so no past operator ever wraps backward.
//!
//! The lasso is unrolled until every past operator has stabilized and the
//! last period is then taken as the loop, which re-expresses the same infinite
//! word. Each result is computed at two unrolling depths and refused if they
//! differ, so an insufficient unrolling cannot go unnoticed.
//!
//! Nothing here calls the root evaluator. Only the shared data types (the
//! formula tree, lasso evidence, limits and outcome) are reused.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    from_syntax, Evidence, Formula, Interval, Lasso, Limits, OracleError, Outcome, Uncertainty,
    Verdict,
};
use tl_syntax::{
    InfiniteFormulaDocument, LassoTraceDocument, NodeId, PropositionId, SemanticProfile,
};

/// One concrete (complete Boolean) lasso cell.
pub type Cell = BTreeMap<PropositionId, bool>;

/// Decides `formula` at `position` on a complete lasso: `cells[..loop_entry]`
/// is the prefix and the rest is the loop, repeated forever.
pub fn decide_complete(
    formula: &Formula,
    cells: &[Cell],
    loop_entry: usize,
    position: usize,
    limits: Limits,
) -> Result<bool, OracleError> {
    if loop_entry >= cells.len() {
        return Err(OracleError::EmptyLoop);
    }
    let weight = weight(formula);
    let period = cells.len() - loop_entry;
    let base = loop_entry + (weight + 2) * period;
    let first = word(cells, loop_entry, base, limits)?.truth(formula, position)?;
    let second = word(cells, loop_entry, base + 3 * period, limits)?.truth(formula, position)?;
    if first == second {
        Ok(first)
    } else {
        Err(OracleError::NoPeriodicFixedPoint)
    }
}

/// Decides a claim over every fair common completion of a partial lasso.
///
/// A completion fixes each missing or conflicting cell once. It is fair when
/// every premise holds at some position of the loop, which is the same as
/// holding at infinitely many positions of the infinite word.
pub fn decide(
    claim: &Formula,
    fairness: &[Formula],
    lasso: &Lasso,
    position: usize,
    limits: Limits,
) -> Result<Outcome, OracleError> {
    let mut atoms = BTreeSet::new();
    let mut depth = claim.collect_atoms(&mut atoms, 1);
    for premise in fairness {
        depth = depth.max(premise.collect_atoms(&mut atoms, 1));
    }
    for cell in &lasso.cells {
        atoms.extend(cell.keys().copied());
        if cell.values().any(|e| *e == Evidence::Impossible) {
            return Err(OracleError::InconsistentInput);
        }
    }
    if depth > limits.max_depth || lasso.len() > limits.max_positions {
        return Err(OracleError::ResourceIncomplete);
    }
    let mut unknown: Vec<(usize, PropositionId)> = Vec::new();
    let (mut missing, mut conflicting) = (false, false);
    for (index, cell) in lasso.cells.iter().enumerate() {
        for atom in &atoms {
            match cell.get(atom) {
                None | Some(Evidence::Missing) => {
                    unknown.push((index, *atom));
                    missing = true;
                }
                Some(Evidence::Conflicting) => {
                    unknown.push((index, *atom));
                    conflicting = true;
                }
                _ => {}
            }
        }
    }
    if unknown.len() >= usize::BITS as usize || (1usize << unknown.len()) > limits.max_completions {
        return Err(OracleError::ResourceIncomplete);
    }

    let (mut can_hold, mut can_fail, mut fair_count) = (false, false, 0usize);
    for choice in 0..(1usize << unknown.len()) {
        let mut cells: Vec<Cell> = lasso
            .cells
            .iter()
            .map(|cell| {
                cell.iter()
                    .filter_map(|(id, e)| match e {
                        Evidence::True => Some((*id, true)),
                        Evidence::False => Some((*id, false)),
                        _ => None,
                    })
                    .collect()
            })
            .collect();
        for (bit, (index, atom)) in unknown.iter().enumerate() {
            cells[*index].insert(*atom, choice >> bit & 1 == 1);
        }
        let mut admitted = true;
        for premise in fairness {
            if !infinitely_often(premise, &cells, lasso.loop_entry, limits)? {
                admitted = false;
                break;
            }
        }
        if !admitted {
            continue;
        }
        fair_count += 1;
        if decide_complete(claim, &cells, lasso.loop_entry, position, limits)? {
            can_hold = true;
        } else {
            can_fail = true;
        }
    }
    let verdict = match (can_hold, can_fail) {
        (true, false) => Verdict::Proved,
        (false, true) => Verdict::Refuted,
        (true, true) => Verdict::Inconclusive,
        (false, false) => return Err(OracleError::EmptyFairAdmission),
    };
    Ok(Outcome {
        verdict,
        possible_true: can_hold,
        possible_false: can_fail,
        fair_completions: fair_count,
        uncertainty: match (missing, conflicting) {
            (false, false) => Uncertainty::None,
            (true, false) => Uncertainty::Missing,
            (false, true) => Uncertainty::Conflicting,
            (true, true) => Uncertainty::Mixed,
        },
    })
}

/// Decides two validated syntax documents, with the same profile, clock and
/// proposition-map admission as the scanning evaluator's document entry point.
pub fn decide_documents(
    formula: &InfiniteFormulaDocument,
    trace: &LassoTraceDocument,
    claim_root: NodeId,
    fairness_roots: &[NodeId],
    position: usize,
    limits: Limits,
) -> Result<Outcome, OracleError> {
    if formula.semantic_profile() != SemanticProfile::InfiniteTraceV1
        || trace.semantic_profile() != SemanticProfile::InfiniteTraceV1
        || formula.clock() != trace.clock()
    {
        return Err(OracleError::IdentityMismatch);
    }
    let graph = formula.formula();
    let claim = from_syntax(graph, claim_root)?;
    let fairness: Vec<_> = fairness_roots
        .iter()
        .map(|root| from_syntax(graph, *root))
        .collect::<Result<_, _>>()?;
    let mut atoms = BTreeSet::new();
    claim.collect_atoms(&mut atoms, 1);
    for premise in &fairness {
        premise.collect_atoms(&mut atoms, 1);
    }
    if !atoms.iter().all(|atom| trace.propositions().contains(atom)) {
        return Err(OracleError::ForeignProposition);
    }
    decide(&claim, &fairness, &Lasso::from(trace), position, limits)
}

/// True when `formula` holds at some position of the loop of the word.
fn infinitely_often(
    formula: &Formula,
    cells: &[Cell],
    loop_entry: usize,
    limits: Limits,
) -> Result<bool, OracleError> {
    let period = cells.len() - loop_entry;
    for offset in 0..period {
        if decide_complete(formula, cells, loop_entry, loop_entry + offset, limits)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// An upper bound on how many loop periods any operator can need to settle.
fn weight(formula: &Formula) -> usize {
    use Formula::*;
    let own = |range: &Interval| match *range {
        Interval::Closed { start, end } => start + end,
        Interval::Unbounded { start } => start + 1,
    };
    match formula {
        False | True | Atom(_) => 0,
        Not(p) => weight(p),
        And(p, q) | Or(p, q) | Implies(p, q) | Equivalent(p, q) => weight(p) + weight(q),
        Future(r, p) | Globally(r, p) | Once(r, p) | Historically(r, p) => own(r) + weight(p),
        Until(r, p, q) | Release(r, p, q) | Since(r, p, q) | Triggered(r, p, q) => {
            own(r) + weight(p) + weight(q)
        }
        StrongPrevious(p) => 1 + weight(p),
    }
}

struct Word<'a> {
    limits: Limits,
    /// Unrolled time points; the last `period` of them are the loop.
    cells: Vec<&'a Cell>,
    period: usize,
}

fn word<'a>(
    cells: &'a [Cell],
    loop_entry: usize,
    length: usize,
    limits: Limits,
) -> Result<Word<'a>, OracleError> {
    if length > limits.max_positions {
        return Err(OracleError::ResourceIncomplete);
    }
    let period = cells.len() - loop_entry;
    let unrolled = (0..length)
        .map(|t| {
            if t < cells.len() {
                &cells[t]
            } else {
                &cells[loop_entry + (t - loop_entry) % period]
            }
        })
        .collect();
    Ok(Word {
        limits,
        cells: unrolled,
        period,
    })
}

impl Word<'_> {
    fn len(&self) -> usize {
        self.cells.len()
    }

    /// Successor along the lasso: the last point returns to the loop start.
    fn next(&self, t: usize) -> usize {
        if t + 1 < self.len() {
            t + 1
        } else {
            self.len() - self.period
        }
    }

    fn truth(&self, formula: &Formula, position: usize) -> Result<bool, OracleError> {
        let values = self.eval(formula)?;
        let loop_start = self.len() - self.period;
        let at = if position < self.len() {
            position
        } else {
            loop_start + (position - loop_start) % self.period
        };
        Ok(values[at])
    }

    fn bounds(&self, range: Interval) -> Result<(usize, Option<usize>), OracleError> {
        let (lower, upper) = match range {
            Interval::Closed { start, end } => (start, Some(end)),
            Interval::Unbounded { start } => (start, None),
        };
        if upper.is_some_and(|end| lower > end) {
            return Err(OracleError::InvertedInterval);
        }
        if lower > self.limits.max_offset || upper.is_some_and(|end| end > self.limits.max_offset) {
            return Err(OracleError::ResourceIncomplete);
        }
        Ok((lower, upper))
    }

    fn constant(&self, value: bool) -> Vec<bool> {
        vec![value; self.len()]
    }

    fn eval(&self, formula: &Formula) -> Result<Vec<bool>, OracleError> {
        use Formula::*;
        let zip = |l: Vec<bool>, r: Vec<bool>, f: fn(bool, bool) -> bool| -> Vec<bool> {
            l.into_iter().zip(r).map(|(a, b)| f(a, b)).collect()
        };
        Ok(match formula {
            True => self.constant(true),
            False => self.constant(false),
            Atom(id) => self
                .cells
                .iter()
                .map(|cell| cell.get(id).copied().unwrap_or(false))
                .collect(),
            Not(p) => self.eval(p)?.into_iter().map(|v| !v).collect(),
            And(p, q) => zip(self.eval(p)?, self.eval(q)?, |a, b| a & b),
            Or(p, q) => zip(self.eval(p)?, self.eval(q)?, |a, b| a | b),
            Implies(p, q) => zip(self.eval(p)?, self.eval(q)?, |a, b| !a | b),
            Equivalent(p, q) => zip(self.eval(p)?, self.eval(q)?, |a, b| a == b),
            Future(r, p) => {
                let (a, b) = self.bounds(*r)?;
                self.until(a, b, &self.constant(true), &self.eval(p)?)
            }
            Globally(r, p) => {
                let (a, b) = self.bounds(*r)?;
                self.release(a, b, &self.constant(false), &self.eval(p)?)
            }
            Until(r, p, q) => {
                let (a, b) = self.bounds(*r)?;
                self.until(a, b, &self.eval(p)?, &self.eval(q)?)
            }
            Release(r, p, q) => {
                let (a, b) = self.bounds(*r)?;
                self.release(a, b, &self.eval(p)?, &self.eval(q)?)
            }
            Once(r, p) => {
                let (a, b) = self.bounds(*r)?;
                self.since(a, b, &self.constant(true), &self.eval(p)?)
            }
            Historically(r, p) => {
                let (a, b) = self.bounds(*r)?;
                self.triggered(a, b, &self.constant(false), &self.eval(p)?)
            }
            Since(r, p, q) => {
                let (a, b) = self.bounds(*r)?;
                self.since(a, b, &self.eval(p)?, &self.eval(q)?)
            }
            Triggered(r, p, q) => {
                let (a, b) = self.bounds(*r)?;
                self.triggered(a, b, &self.eval(p)?, &self.eval(q)?)
            }
            StrongPrevious(p) => self.since(1, Some(1), &self.constant(true), &self.eval(p)?),
        })
    }

    /// `p U[a,b] q` by the successor equations.
    fn until(&self, a: usize, b: Option<usize>, p: &[bool], q: &[bool]) -> Vec<bool> {
        let n = self.len();
        if a > 0 {
            let rest = self.until(a - 1, b.map(|e| e - 1), p, q);
            return (0..n).map(|i| p[i] && rest[self.next(i)]).collect();
        }
        match b {
            Some(0) => q.to_vec(),
            Some(end) => {
                let rest = self.until(0, Some(end - 1), p, q);
                (0..n)
                    .map(|i| q[i] || (p[i] && rest[self.next(i)]))
                    .collect()
            }
            None => self.solve(false, |i, x| q[i] || (p[i] && x[self.next(i)])),
        }
    }

    /// `p R[a,b] q` by the successor equations.
    fn release(&self, a: usize, b: Option<usize>, p: &[bool], q: &[bool]) -> Vec<bool> {
        let n = self.len();
        if a > 0 {
            let rest = self.release(a - 1, b.map(|e| e - 1), p, q);
            return (0..n).map(|i| p[i] || rest[self.next(i)]).collect();
        }
        match b {
            Some(0) => q.to_vec(),
            Some(end) => {
                let rest = self.release(0, Some(end - 1), p, q);
                (0..n)
                    .map(|i| q[i] && (p[i] || rest[self.next(i)]))
                    .collect()
            }
            None => self.solve(true, |i, x| q[i] && (p[i] || x[self.next(i)])),
        }
    }

    /// `p S[a,b] q`, stepping to the previous time point.
    fn since(&self, a: usize, b: Option<usize>, p: &[bool], q: &[bool]) -> Vec<bool> {
        let n = self.len();
        if a > 0 {
            let rest = self.since(a - 1, b.map(|e| e - 1), p, q);
            return (0..n).map(|t| t >= 1 && p[t] && rest[t - 1]).collect();
        }
        match b {
            Some(0) => q.to_vec(),
            Some(end) => {
                let rest = self.since(0, Some(end - 1), p, q);
                (0..n)
                    .map(|t| q[t] || (p[t] && t >= 1 && rest[t - 1]))
                    .collect()
            }
            None => {
                let mut x = vec![false; n];
                for t in 0..n {
                    x[t] = q[t] || (p[t] && t >= 1 && x[t - 1]);
                }
                x
            }
        }
    }

    /// `p T[a,b] q`, stepping to the previous time point.
    fn triggered(&self, a: usize, b: Option<usize>, p: &[bool], q: &[bool]) -> Vec<bool> {
        let n = self.len();
        if a > 0 {
            let rest = self.triggered(a - 1, b.map(|e| e - 1), p, q);
            return (0..n).map(|t| t < 1 || p[t] || rest[t - 1]).collect();
        }
        match b {
            Some(0) => q.to_vec(),
            Some(end) => {
                let rest = self.triggered(0, Some(end - 1), p, q);
                (0..n)
                    .map(|t| q[t] && (t < 1 || p[t] || rest[t - 1]))
                    .collect()
            }
            None => {
                let mut x = vec![true; n];
                for t in 0..n {
                    x[t] = q[t] && (t < 1 || p[t] || x[t - 1]);
                }
                x
            }
        }
    }

    /// Iterates `x'[i] = step(i, x)` from `start` until it stops changing.
    fn solve(&self, start: bool, step: impl Fn(usize, &[bool]) -> bool) -> Vec<bool> {
        let mut x = self.constant(start);
        loop {
            let y: Vec<bool> = (0..self.len()).map(|i| step(i, &x)).collect();
            if y == x {
                return x;
            }
            x = y;
        }
    }
}
