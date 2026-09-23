//! Small-scope reference interpretations for the two bounded owner profiles.

use std::collections::BTreeMap;

use tl_syntax::PropositionId;

use crate::{Formula, Interval, Limits, OracleError};

#[derive(Clone, Copy)]
enum Domain {
    ClosedFuture,
    OriginPast,
}

/// Evaluates a complete finite word under `mltl.closed-trace/v1`.
///
/// Missing or out-of-word atomic observations are false, while Boolean
/// constants remain time independent. For bounded U/R, the left guard starts
/// at the interval's lower endpoint. Past operators are refused.
pub fn evaluate_closed_trace_v1(
    formula: &Formula,
    cells: &[BTreeMap<PropositionId, bool>],
    position: usize,
    limits: Limits,
) -> Result<bool, OracleError> {
    evaluate(formula, cells, position, limits, Domain::ClosedFuture)
}

/// Evaluates a finite history under `mltl.origin-complete-history/v1`.
///
/// Atomic observations before the origin are false; Boolean constants and
/// connectives still evaluate normally there. O/H/Y/S/T visit the full closed
/// interval, including negative positions. Missing atoms in a history cell
/// are false. Future operators are refused.
pub fn evaluate_origin_complete(
    formula: &Formula,
    cells: &[BTreeMap<PropositionId, bool>],
    position: usize,
    limits: Limits,
) -> Result<bool, OracleError> {
    evaluate(formula, cells, position, limits, Domain::OriginPast)
}

fn evaluate(
    formula: &Formula,
    cells: &[BTreeMap<PropositionId, bool>],
    position: usize,
    limits: Limits,
    domain: Domain,
) -> Result<bool, OracleError> {
    if position >= cells.len() {
        return Err(OracleError::FinitePositionAbsent);
    }
    if cells.len() > limits.max_positions {
        return Err(OracleError::ResourceIncomplete);
    }
    let mut budget = limits.max_positions;
    let position = i128::try_from(position).map_err(|_| OracleError::ResourceIncomplete)?;
    let mut evaluator = Reference {
        cells,
        limits,
        domain,
        budget: &mut budget,
    };
    evaluator.check(formula, 1)?;
    evaluator.at(formula, position)
}

struct Reference<'a> {
    cells: &'a [BTreeMap<PropositionId, bool>],
    limits: Limits,
    domain: Domain,
    budget: &'a mut usize,
}

impl Reference<'_> {
    fn interval(&self, interval: Interval) -> Result<(usize, usize), OracleError> {
        match interval {
            Interval::Unbounded { .. } => Err(OracleError::UnboundedFiniteInterval),
            Interval::Closed { start, end } if start > end => Err(OracleError::InvertedInterval),
            Interval::Closed { end, .. } if end > self.limits.max_offset => {
                Err(OracleError::ResourceIncomplete)
            }
            Interval::Closed { start, end } => Ok((start, end)),
        }
    }

    fn check(&self, formula: &Formula, depth: usize) -> Result<(), OracleError> {
        use Formula::*;
        if depth > self.limits.max_depth {
            return Err(OracleError::ResourceIncomplete);
        }
        let next = depth
            .checked_add(1)
            .ok_or(OracleError::ResourceIncomplete)?;
        match formula {
            False | True | Atom(_) => Ok(()),
            Not(p) | StrongPrevious(p) => {
                if matches!(formula, StrongPrevious(_))
                    && matches!(self.domain, Domain::ClosedFuture)
                {
                    return Err(OracleError::ClosedTracePastUnsupported);
                }
                self.check(p, next)
            }
            And(p, q) | Or(p, q) | Implies(p, q) | Equivalent(p, q) => {
                self.check(p, next)?;
                self.check(q, next)
            }
            Future(range, p) | Globally(range, p) => {
                if matches!(self.domain, Domain::OriginPast) {
                    return Err(OracleError::OriginFutureUnsupported);
                }
                self.interval(*range)?;
                self.check(p, next)
            }
            Until(range, p, q) | Release(range, p, q) => {
                if matches!(self.domain, Domain::OriginPast) {
                    return Err(OracleError::OriginFutureUnsupported);
                }
                self.interval(*range)?;
                self.check(p, next)?;
                self.check(q, next)
            }
            Once(range, p) | Historically(range, p) => {
                if matches!(self.domain, Domain::ClosedFuture) {
                    return Err(OracleError::ClosedTracePastUnsupported);
                }
                self.interval(*range)?;
                self.check(p, next)
            }
            Since(range, p, q) | Triggered(range, p, q) => {
                if matches!(self.domain, Domain::ClosedFuture) {
                    return Err(OracleError::ClosedTracePastUnsupported);
                }
                self.interval(*range)?;
                self.check(p, next)?;
                self.check(q, next)
            }
        }
    }

    fn tick(&mut self) -> Result<(), OracleError> {
        *self.budget = self
            .budget
            .checked_sub(1)
            .ok_or(OracleError::ResourceIncomplete)?;
        Ok(())
    }

    fn shifted(time: i128, offset: usize, past: bool) -> Result<i128, OracleError> {
        let offset = i128::try_from(offset).map_err(|_| OracleError::ResourceIncomplete)?;
        if past {
            time.checked_sub(offset)
        } else {
            time.checked_add(offset)
        }
        .ok_or(OracleError::ResourceIncomplete)
    }

    fn witness(
        &mut self,
        interval: Interval,
        left: Option<&Formula>,
        right: &Formula,
        time: i128,
        past: bool,
    ) -> Result<bool, OracleError> {
        let (start, end) = self.interval(interval)?;
        for witness in start..=end {
            let witness_time = Self::shifted(time, witness, past)?;
            if !self.at(right, witness_time)? {
                continue;
            }
            let mut all_guards = true;
            if let Some(left) = left {
                for offset in start..witness {
                    if !self.at(left, Self::shifted(time, offset, past)?)? {
                        all_guards = false;
                        break;
                    }
                }
            }
            if all_guards {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn at(&mut self, formula: &Formula, time: i128) -> Result<bool, OracleError> {
        use Formula::*;
        self.tick()?;
        match formula {
            False => Ok(false),
            True => Ok(true),
            Atom(id) => Ok(usize::try_from(time)
                .ok()
                .and_then(|index| self.cells.get(index))
                .and_then(|cell| cell.get(id))
                .copied()
                .unwrap_or(false)),
            Not(p) => Ok(!self.at(p, time)?),
            And(p, q) => Ok(self.at(p, time)? && self.at(q, time)?),
            Or(p, q) => Ok(self.at(p, time)? || self.at(q, time)?),
            Implies(p, q) => Ok(!self.at(p, time)? || self.at(q, time)?),
            Equivalent(p, q) => Ok(self.at(p, time)? == self.at(q, time)?),
            Future(range, p) | Once(range, p) => {
                self.witness(*range, None, p, time, matches!(formula, Once(_, _)))
            }
            Globally(range, p) | Historically(range, p) => Ok(!self.witness(
                *range,
                None,
                &Not(p.clone()),
                time,
                matches!(formula, Historically(_, _)),
            )?),
            Until(range, p, q) | Since(range, p, q) => {
                self.witness(*range, Some(p), q, time, matches!(formula, Since(_, _, _)))
            }
            Release(range, p, q) | Triggered(range, p, q) => Ok(!self.witness(
                *range,
                Some(&Not(p.clone())),
                &Not(q.clone()),
                time,
                matches!(formula, Triggered(_, _, _)),
            )?),
            StrongPrevious(p) => self.at(p, Self::shifted(time, 1, true)?),
        }
    }
}
