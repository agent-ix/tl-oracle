//! Exact accounting for a declared, contiguous finite verification partition.
//!
//! The caller owns the independent expected answer. This ledger checks that
//! every declared case was visited once and that classifications agree; it
//! cannot establish the independence of the caller's reference implementation.

use std::collections::BTreeSet;

use thiserror::Error;

/// The result of one case, keeping an admitted verdict apart from a refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseResult<V> {
    /// The profile admits this case and produces a verdict.
    Admitted(V),
    /// This formula/trace/profile combination is outside the partition's semantics.
    Refused,
}

/// Exact counts for a completed partition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PopulationSummary {
    /// Number of declared cases.
    pub declared: u64,
    /// Number of admitted cases checked against a reference.
    pub admitted: u64,
    /// Number of explicitly refused cases.
    pub refused: u64,
}

/// A population integrity or comparison failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum PopulationError {
    /// The partition must declare at least one case.
    #[error("the declared population is empty")]
    Empty,
    /// A case identifier lies outside `0..declared`.
    #[error("case identifier {case_id} is outside the declared population")]
    OutOfDomain { case_id: u64 },
    /// The same case was visited more than once.
    #[error("case identifier {case_id} was visited twice")]
    Duplicate { case_id: u64 },
    /// The observed verdict or refusal does not match the reference.
    #[error("case identifier {case_id} disagrees with its independent reference")]
    Mismatch { case_id: u64 },
    /// At least one declared case was never visited.
    #[error("visited {visited} of {declared} declared cases")]
    Incomplete { declared: u64, visited: u64 },
}

/// Records exactly one visit for every ID in `0..declared`.
#[derive(Debug)]
pub struct PopulationLedger {
    declared: u64,
    seen: BTreeSet<u64>,
    admitted: u64,
    refused: u64,
    first_failure: Option<PopulationError>,
}

impl PopulationLedger {
    /// Creates a partition with an independently calculated cardinality.
    pub fn new(declared: u64) -> Result<Self, PopulationError> {
        if declared == 0 {
            return Err(PopulationError::Empty);
        }
        Ok(Self {
            declared,
            seen: BTreeSet::new(),
            admitted: 0,
            refused: 0,
            first_failure: None,
        })
    }

    /// Records a visit and compares its classification to an independent reference.
    pub fn record<V: Eq>(
        &mut self,
        case_id: u64,
        reference: CaseResult<V>,
        observed: CaseResult<V>,
    ) -> Result<(), PopulationError> {
        if case_id >= self.declared {
            let error = PopulationError::OutOfDomain { case_id };
            self.first_failure.get_or_insert(error);
            return Err(error);
        }
        if !self.seen.insert(case_id) {
            let error = PopulationError::Duplicate { case_id };
            self.first_failure.get_or_insert(error);
            return Err(error);
        }
        if reference != observed {
            let error = PopulationError::Mismatch { case_id };
            self.first_failure.get_or_insert(error);
            return Err(error);
        }
        match reference {
            CaseResult::Admitted(_) => self.admitted += 1,
            CaseResult::Refused => self.refused += 1,
        }
        Ok(())
    }

    /// Completes the partition only if all declared IDs were visited once.
    pub fn finish(self) -> Result<PopulationSummary, PopulationError> {
        if let Some(error) = self.first_failure {
            return Err(error);
        }
        let visited = u64::try_from(self.seen.len()).expect("seen count fits u64");
        if visited != self.declared {
            return Err(PopulationError::Incomplete {
                declared: self.declared,
                visited,
            });
        }
        Ok(PopulationSummary {
            declared: self.declared,
            admitted: self.admitted,
            refused: self.refused,
        })
    }
}
