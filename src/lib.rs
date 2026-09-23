//! Development-only reference semantics for finite and ultimately periodic TL words.
//!
//! This crate imports syntax identities only. Its recursive denotation is independent
//! of the production evaluators and rewriter. All verdicts are trace scoped.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;
use tl_syntax::{
    InfiniteFormula, InfiniteFormulaDocument, InfiniteNodeKind, LassoTraceDocument, NodeId,
    PartialValue, PropositionId, SemanticProfile, TemporalInterval,
};

/// Counted verification partitions used by downstream development tests.
pub mod population;

mod profile;
pub use profile::{evaluate_closed_trace_v1, evaluate_origin_complete};

/// A discrete inclusive interval or an interval extending without an end.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interval {
    /// Offsets from `start` through `end`, inclusive.
    Closed { start: usize, end: usize },
    /// All offsets at least `start`.
    Unbounded { start: usize },
}

impl Interval {
    fn bounds(self) -> (usize, Option<usize>) {
        match self {
            Self::Closed { start, end } => (start, Some(end)),
            Self::Unbounded { start } => (start, None),
        }
    }
}

/// Oracle-owned formula tree. Construction never imports production evaluation code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Formula {
    /// Boolean false.
    False,
    /// Boolean true.
    True,
    /// A proposition from the public syntax contract.
    Atom(PropositionId),
    /// Boolean negation.
    Not(Box<Self>),
    /// Boolean conjunction.
    And(Box<Self>, Box<Self>),
    /// Boolean disjunction.
    Or(Box<Self>, Box<Self>),
    /// Boolean implication.
    Implies(Box<Self>, Box<Self>),
    /// Boolean equivalence.
    Equivalent(Box<Self>, Box<Self>),
    /// Future F.
    Future(Interval, Box<Self>),
    /// Future G.
    Globally(Interval, Box<Self>),
    /// Future U.
    Until(Interval, Box<Self>, Box<Self>),
    /// Future R.
    Release(Interval, Box<Self>, Box<Self>),
    /// Past O.
    Once(Interval, Box<Self>),
    /// Past H.
    Historically(Interval, Box<Self>),
    /// Past S.
    Since(Interval, Box<Self>, Box<Self>),
    /// Past T.
    Triggered(Interval, Box<Self>, Box<Self>),
    /// Strong previous Y.
    StrongPrevious(Box<Self>),
}

impl Formula {
    fn collect_atoms(&self, atoms: &mut BTreeSet<PropositionId>, depth: usize) -> usize {
        use Formula::*;
        match self {
            False | True => depth,
            Atom(id) => {
                atoms.insert(*id);
                depth
            }
            Not(p)
            | Future(_, p)
            | Globally(_, p)
            | Once(_, p)
            | Historically(_, p)
            | StrongPrevious(p) => p.collect_atoms(atoms, depth + 1),
            And(p, q)
            | Or(p, q)
            | Implies(p, q)
            | Equivalent(p, q)
            | Until(_, p, q)
            | Release(_, p, q)
            | Since(_, p, q)
            | Triggered(_, p, q) => p
                .collect_atoms(atoms, depth + 1)
                .max(q.collect_atoms(atoms, depth + 1)),
        }
    }
}

/// Evidence for one proposition at one materialized lasso position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Evidence {
    /// The proposition is known false.
    False,
    /// The proposition is known true.
    True,
    /// No observation has arrived.
    Missing,
    /// Conflicting true and false observations exist.
    Conflicting,
    /// An explicitly empty possibility set; the input is inconsistent.
    Impossible,
}

/// A prefix followed by a nonempty loop, with proposition evidence at each cell.
#[derive(Clone, Debug)]
pub struct Lasso {
    cells: Vec<BTreeMap<PropositionId, Evidence>>,
    loop_entry: usize,
}

impl Lasso {
    /// Creates a lasso. A position absent from a cell is missing evidence.
    pub fn new(
        prefix: Vec<BTreeMap<PropositionId, Evidence>>,
        loop_cells: Vec<BTreeMap<PropositionId, Evidence>>,
    ) -> Result<Self, OracleError> {
        if loop_cells.is_empty() {
            return Err(OracleError::EmptyLoop);
        }
        let loop_entry = prefix.len();
        let mut cells = prefix;
        cells.extend(loop_cells);
        Ok(Self { cells, loop_entry })
    }

    fn len(&self) -> usize {
        self.cells.len()
    }

    /// Index where the repeating loop begins.
    pub fn loop_entry(&self) -> usize {
        self.loop_entry
    }
}

impl From<&LassoTraceDocument> for Lasso {
    fn from(trace: &LassoTraceDocument) -> Self {
        fn cell(observation: &tl_syntax::TraceObservation) -> BTreeMap<PropositionId, Evidence> {
            observation
                .valuation
                .entries()
                .iter()
                .map(|entry| {
                    let evidence = match entry.value {
                        PartialValue::False => Evidence::False,
                        PartialValue::True => Evidence::True,
                        PartialValue::Missing => Evidence::Missing,
                        PartialValue::Conflicting => Evidence::Conflicting,
                    };
                    (entry.proposition, evidence)
                })
                .collect()
        }
        Self {
            cells: trace
                .prefix()
                .iter()
                .chain(trace.loop_observations())
                .map(cell)
                .collect(),
            loop_entry: trace.loop_entry(),
        }
    }
}

/// Maximum work admitted by one oracle request.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum formula nesting depth.
    pub max_depth: usize,
    /// Maximum temporal offset, including a past stabilization prefix.
    pub max_offset: usize,
    /// Maximum sequence cells materialized for any subformula.
    pub max_positions: usize,
    /// Maximum common completions enumerated.
    pub max_completions: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_offset: 4096,
            max_positions: 65536,
            max_completions: 65536,
        }
    }
}

/// A trace-scoped semantic result, never a model-wide proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// All fair admitted completions satisfy the claim.
    Proved,
    /// All fair admitted completions falsify the claim.
    Refuted,
    /// Fair admitted completions disagree.
    Inconclusive,
}

/// Why an inconclusive result can remain unsettled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Uncertainty {
    /// No unknown evidence occurred.
    None,
    /// At least one observation was missing.
    Missing,
    /// At least one observation was conflicting.
    Conflicting,
    /// Both missing and conflicting evidence occurred.
    Mixed,
}

/// Exact result of complete common-completion enumeration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Outcome {
    /// Truth settlement for one trace at the requested position.
    pub verdict: Verdict,
    /// Whether at least one admitted fair completion made the claim true.
    pub possible_true: bool,
    /// Whether at least one admitted fair completion made the claim false.
    pub possible_false: bool,
    /// Count of admitted fair completions.
    pub fair_completions: usize,
    /// Source of open possibilities, even if the formula happened to settle.
    pub uncertainty: Uncertainty,
}

/// Refusal or resource-incomplete result. Neither is a Boolean truth value.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum OracleError {
    /// A lasso needs at least one loop cell.
    #[error("lasso loop is empty")]
    EmptyLoop,
    /// A closed interval was inverted.
    #[error("closed interval start exceeds end")]
    InvertedInterval,
    /// One observed cell has no consistent Boolean completion.
    #[error("cell has an empty proposition possibility set")]
    InconsistentInput,
    /// Fairness rejected every common completion.
    #[error("no completion satisfies every fairness premise")]
    EmptyFairAdmission,
    /// The configured resource budget was exhausted.
    #[error("oracle resource limit exceeded")]
    ResourceIncomplete,
    /// The reference sequence did not stabilize within the configured bound.
    #[error("past recurrence did not stabilize within the configured bound")]
    NoPeriodicFixedPoint,
    /// A requested root does not belong to the validated syntax graph.
    #[error("requested root is absent from the syntax graph")]
    GraphRootAbsent,
    /// A finite trace cannot be evaluated at an absent starting position.
    #[error("requested finite-trace position is absent")]
    FinitePositionAbsent,
    /// The finite oracle needs a value for each referenced proposition.
    #[error("finite trace omits a referenced proposition")]
    FinitePropositionMissing,
    /// A past operator was supplied to the closed future profile.
    #[error("past operator is unsupported by the closed-trace profile")]
    ClosedTracePastUnsupported,
    /// A future operator was supplied to the origin-complete past profile.
    #[error("future operator is unsupported by the origin-complete profile")]
    OriginFutureUnsupported,
    /// An open-upper interval is outside the closed finite profile.
    #[error("unbounded interval is not admitted on a closed finite trace")]
    UnboundedFiniteInterval,
    /// The formula and trace do not share the infinite profile and event clock.
    #[error("formula and trace profile or clock disagree")]
    IdentityMismatch,
    /// The trace's proposition map does not admit a referenced atom.
    #[error("formula references an atom outside the trace proposition map")]
    ForeignProposition,
}

/// Converts one root of a validated public syntax graph to the oracle tree.
///
/// This copies syntax shape only; no evaluator or rewrite implementation is
/// imported. Every operator variant is matched exhaustively.
pub fn from_syntax(graph: InfiniteFormula<'_>, root: NodeId) -> Result<Formula, OracleError> {
    fn lower(
        graph: InfiniteFormula<'_>,
        root: NodeId,
        depth: usize,
        budget: &mut usize,
    ) -> Result<Formula, OracleError> {
        if depth >= 64 || *budget == 0 {
            return Err(OracleError::ResourceIncomplete);
        }
        *budget -= 1;
        let mut child = |id| lower(graph, id, depth + 1, budget).map(Box::new);
        fn interval(value: TemporalInterval) -> Result<Interval, OracleError> {
            match value {
                TemporalInterval::Closed(range) => Ok(Interval::Closed {
                    start: usize::try_from(range.start())
                        .map_err(|_| OracleError::ResourceIncomplete)?,
                    end: usize::try_from(range.end())
                        .map_err(|_| OracleError::ResourceIncomplete)?,
                }),
                TemporalInterval::Unbounded(range) => Ok(Interval::Unbounded {
                    start: usize::try_from(range.start())
                        .map_err(|_| OracleError::ResourceIncomplete)?,
                }),
            }
        }
        let node = graph.node(root).ok_or(OracleError::GraphRootAbsent)?;
        let result = match node.kind {
            InfiniteNodeKind::False => Formula::False,
            InfiniteNodeKind::True => Formula::True,
            InfiniteNodeKind::Proposition { proposition } => Formula::Atom(proposition),
            InfiniteNodeKind::Not { operand } => Formula::Not(child(operand)?),
            InfiniteNodeKind::And { left, right } => Formula::And(child(left)?, child(right)?),
            InfiniteNodeKind::Or { left, right } => Formula::Or(child(left)?, child(right)?),
            InfiniteNodeKind::Implies { left, right } => {
                Formula::Implies(child(left)?, child(right)?)
            }
            InfiniteNodeKind::Equivalent { left, right } => {
                Formula::Equivalent(child(left)?, child(right)?)
            }
            InfiniteNodeKind::Future {
                interval: r,
                operand,
            } => Formula::Future(interval(r)?, child(operand)?),
            InfiniteNodeKind::Globally {
                interval: r,
                operand,
            } => Formula::Globally(interval(r)?, child(operand)?),
            InfiniteNodeKind::Until {
                interval: r,
                left,
                right,
            } => Formula::Until(interval(r)?, child(left)?, child(right)?),
            InfiniteNodeKind::Release {
                interval: r,
                left,
                right,
            } => Formula::Release(interval(r)?, child(left)?, child(right)?),
            InfiniteNodeKind::Once {
                interval: r,
                operand,
            } => Formula::Once(interval(r)?, child(operand)?),
            InfiniteNodeKind::Historically {
                interval: r,
                operand,
            } => Formula::Historically(interval(r)?, child(operand)?),
            InfiniteNodeKind::StrongPrevious { operand } => {
                Formula::StrongPrevious(child(operand)?)
            }
            InfiniteNodeKind::Since {
                interval: r,
                left,
                right,
            } => Formula::Since(interval(r)?, child(left)?, child(right)?),
            InfiniteNodeKind::Triggered {
                interval: r,
                left,
                right,
            } => Formula::Triggered(interval(r)?, child(left)?, child(right)?),
        };
        Ok(result)
    }
    let mut budget = 65_536;
    lower(graph, root, 0, &mut budget)
}

/// Evaluates a complete finite word with the oracle's original temporal clauses.
///
/// Atomic observations beyond the closed word are false; Boolean constants
/// keep their usual values. Every referenced atom in a materialized cell must
/// be present. Past windows stop at the origin and U/R/S/T guards begin at
/// offset zero. Open-upper intervals are refused. Use the explicit
/// [`evaluate_closed_trace_v1`] or [`evaluate_origin_complete`] entry points
/// when comparing against those owner profiles.
pub fn evaluate_finite(
    formula: &Formula,
    cells: &[BTreeMap<PropositionId, bool>],
    position: usize,
    limits: Limits,
) -> Result<bool, OracleError> {
    if position >= cells.len() {
        return Err(OracleError::FinitePositionAbsent);
    }
    let mut atoms = BTreeSet::new();
    if formula.collect_atoms(&mut atoms, 1) > limits.max_depth || cells.len() > limits.max_positions
    {
        return Err(OracleError::ResourceIncomplete);
    }
    for cell in cells {
        for atom in &atoms {
            if !cell.contains_key(atom) {
                return Err(OracleError::FinitePropositionMissing);
            }
        }
    }
    fn bounds(range: Interval, limits: Limits) -> Result<(usize, usize), OracleError> {
        match range {
            Interval::Unbounded { .. } => Err(OracleError::UnboundedFiniteInterval),
            Interval::Closed { start, end } if start > end => Err(OracleError::InvertedInterval),
            Interval::Closed { end, .. } if end > limits.max_offset => {
                Err(OracleError::ResourceIncomplete)
            }
            Interval::Closed { start, end } => Ok((start, end)),
        }
    }
    fn preflight(formula: &Formula, limits: Limits) -> Result<(), OracleError> {
        use Formula::*;
        match formula {
            False | True | Atom(_) => Ok(()),
            Not(p) | StrongPrevious(p) => preflight(p, limits),
            And(p, q) | Or(p, q) | Implies(p, q) | Equivalent(p, q) => {
                preflight(p, limits)?;
                preflight(q, limits)
            }
            Future(range, p) | Globally(range, p) | Once(range, p) | Historically(range, p) => {
                bounds(*range, limits)?;
                preflight(p, limits)
            }
            Until(range, p, q)
            | Release(range, p, q)
            | Since(range, p, q)
            | Triggered(range, p, q) => {
                bounds(*range, limits)?;
                preflight(p, limits)?;
                preflight(q, limits)
            }
        }
    }
    fn future(
        range: Interval,
        witness: &Formula,
        guard: Option<&Formula>,
        cells: &[BTreeMap<PropositionId, bool>],
        time: usize,
        limits: Limits,
    ) -> Result<bool, OracleError> {
        let (lower, upper) = bounds(range, limits)?;
        for offset in 0..=upper {
            let instant = time
                .checked_add(offset)
                .ok_or(OracleError::ResourceIncomplete)?;
            if offset >= lower && at(witness, cells, instant, limits)? {
                return Ok(true);
            }
            if let Some(guard) = guard {
                if !at(guard, cells, instant, limits)? {
                    return Ok(false);
                }
            }
        }
        Ok(false)
    }
    fn past(
        range: Interval,
        witness: &Formula,
        guard: Option<&Formula>,
        cells: &[BTreeMap<PropositionId, bool>],
        time: usize,
        limits: Limits,
    ) -> Result<bool, OracleError> {
        let (lower, upper) = bounds(range, limits)?;
        for offset in 0..=upper.min(time) {
            if offset >= lower && at(witness, cells, time - offset, limits)? {
                return Ok(true);
            }
            if let Some(guard) = guard {
                if !at(guard, cells, time - offset, limits)? {
                    return Ok(false);
                }
            }
        }
        Ok(false)
    }
    fn at(
        formula: &Formula,
        cells: &[BTreeMap<PropositionId, bool>],
        time: usize,
        limits: Limits,
    ) -> Result<bool, OracleError> {
        use Formula::*;
        match formula {
            False => Ok(false),
            True => Ok(true),
            Atom(id) => Ok(cells
                .get(time)
                .and_then(|cell| cell.get(id))
                .copied()
                .unwrap_or(false)),
            Not(p) => Ok(!at(p, cells, time, limits)?),
            And(p, q) => {
                let left = at(p, cells, time, limits)?;
                let right = at(q, cells, time, limits)?;
                Ok(left && right)
            }
            Or(p, q) => {
                let left = at(p, cells, time, limits)?;
                let right = at(q, cells, time, limits)?;
                Ok(left || right)
            }
            Implies(p, q) => {
                let left = at(p, cells, time, limits)?;
                let right = at(q, cells, time, limits)?;
                Ok(!left || right)
            }
            Equivalent(p, q) => {
                let left = at(p, cells, time, limits)?;
                let right = at(q, cells, time, limits)?;
                Ok(left == right)
            }
            Future(range, p) => future(*range, p, None, cells, time, limits),
            Globally(range, p) => Ok(!future(*range, &Not(p.clone()), None, cells, time, limits)?),
            Until(range, p, q) => future(*range, q, Some(p), cells, time, limits),
            Release(range, p, q) => Ok(!future(
                *range,
                &Not(q.clone()),
                Some(&Not(p.clone())),
                cells,
                time,
                limits,
            )?),
            Once(range, p) => past(*range, p, None, cells, time, limits),
            Historically(range, p) => {
                Ok(!past(*range, &Not(p.clone()), None, cells, time, limits)?)
            }
            Since(range, p, q) => past(*range, q, Some(p), cells, time, limits),
            Triggered(range, p, q) => Ok(!past(
                *range,
                &Not(q.clone()),
                Some(&Not(p.clone())),
                cells,
                time,
                limits,
            )?),
            StrongPrevious(p) => past(
                Interval::Closed { start: 1, end: 1 },
                p,
                None,
                cells,
                time,
                limits,
            ),
        }
    }
    preflight(formula, limits)?;
    at(formula, cells, position, limits)
}

/// Evaluates roots of the same validated syntax graph against one trace.
pub fn evaluate_syntax(
    graph: InfiniteFormula<'_>,
    claim_root: NodeId,
    fairness_roots: &[NodeId],
    lasso: &Lasso,
    position: usize,
    limits: Limits,
) -> Result<Outcome, OracleError> {
    let claim = from_syntax(graph, claim_root)?;
    let fairness: Vec<_> = fairness_roots
        .iter()
        .map(|root| from_syntax(graph, *root))
        .collect::<Result<_, _>>()?;
    evaluate(&claim, &fairness, lasso, position, limits)
}

/// Evaluates two validated public syntax documents with exact profile, clock,
/// graph-root and proposition-map admission before semantic interpretation.
pub fn evaluate_documents(
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
    evaluate(&claim, &fairness, &Lasso::from(trace), position, limits)
}

/// Evaluates a claim at `position` on one lasso, with fairness filtering.
///
/// Missing and conflicting cells each enumerate two Boolean values but retain
/// different reasons. A completion fixes each cell once across all references
/// and all repetitions of the loop. This function never settles a model claim.
pub fn evaluate(
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
        if cell.values().any(|value| *value == Evidence::Impossible) {
            return Err(OracleError::InconsistentInput);
        }
    }
    if depth > limits.max_depth || lasso.len() > limits.max_positions {
        return Err(OracleError::ResourceIncomplete);
    }
    let mut unknown = Vec::new();
    let mut missing = false;
    let mut conflicting = false;
    for (cell_index, cell) in lasso.cells.iter().enumerate() {
        for atom in &atoms {
            match cell.get(atom).copied().unwrap_or(Evidence::Missing) {
                Evidence::Missing => {
                    unknown.push((cell_index, *atom));
                    missing = true;
                }
                Evidence::Conflicting => {
                    unknown.push((cell_index, *atom));
                    conflicting = true;
                }
                Evidence::Impossible => return Err(OracleError::InconsistentInput),
                Evidence::False | Evidence::True => {}
            }
        }
    }
    let completions = 1usize
        .checked_shl(u32::try_from(unknown.len()).map_err(|_| OracleError::ResourceIncomplete)?)
        .ok_or(OracleError::ResourceIncomplete)?;
    if completions > limits.max_completions {
        return Err(OracleError::ResourceIncomplete);
    }

    let mut possible_true = false;
    let mut possible_false = false;
    let mut fair_completions = 0usize;
    for mask in 0..completions {
        let mut cells: Vec<BTreeMap<PropositionId, bool>> = lasso
            .cells
            .iter()
            .map(|cell| {
                cell.iter()
                    .filter_map(|(id, value)| match value {
                        Evidence::False => Some((*id, false)),
                        Evidence::True => Some((*id, true)),
                        Evidence::Missing | Evidence::Conflicting | Evidence::Impossible => None,
                    })
                    .collect()
            })
            .collect();
        for (bit, (cell, atom)) in unknown.iter().enumerate() {
            cells[*cell].insert(*atom, (mask & (1usize << bit)) != 0);
        }
        let word = ConcreteWord {
            cells: &cells,
            loop_entry: lasso.loop_entry,
            limits,
        };
        let fair = fairness.iter().try_fold(true, |admitted, premise| {
            if !admitted {
                return Ok(false);
            }
            let sequence = word.eval(premise)?;
            Ok::<bool, OracleError>(
                (sequence.start..sequence.start + word.period()).any(|index| sequence.at(index)),
            )
        })?;
        if !fair {
            continue;
        }
        fair_completions += 1;
        let value = word.eval(claim)?.at(position);
        possible_true |= value;
        possible_false |= !value;
    }
    if fair_completions == 0 {
        return Err(OracleError::EmptyFairAdmission);
    }
    let verdict = match (possible_false, possible_true) {
        (false, true) => Verdict::Proved,
        (true, false) => Verdict::Refuted,
        (true, true) => Verdict::Inconclusive,
        (false, false) => return Err(OracleError::EmptyFairAdmission),
    };
    let uncertainty = match (missing, conflicting) {
        (false, false) => Uncertainty::None,
        (true, false) => Uncertainty::Missing,
        (false, true) => Uncertainty::Conflicting,
        (true, true) => Uncertainty::Mixed,
    };
    Ok(Outcome {
        verdict,
        possible_true,
        possible_false,
        fair_completions,
        uncertainty,
    })
}

#[derive(Clone)]
struct Sequence {
    values: Vec<bool>,
    start: usize,
    period: usize,
}

impl Sequence {
    fn at(&self, index: usize) -> bool {
        let index = if index < self.start {
            index
        } else {
            self.start + (index - self.start) % self.period
        };
        self.values[index]
    }

    fn negated(mut self) -> Self {
        for value in &mut self.values {
            *value = !*value;
        }
        self
    }
}

struct ConcreteWord<'a> {
    cells: &'a [BTreeMap<PropositionId, bool>],
    loop_entry: usize,
    limits: Limits,
}

impl ConcreteWord<'_> {
    fn period(&self) -> usize {
        self.cells.len() - self.loop_entry
    }

    fn sequence(
        &self,
        start: usize,
        mut at: impl FnMut(usize) -> bool,
    ) -> Result<Sequence, OracleError> {
        let len = start
            .checked_add(self.period())
            .ok_or(OracleError::ResourceIncomplete)?;
        if len > self.limits.max_positions {
            return Err(OracleError::ResourceIncomplete);
        }
        Ok(Sequence {
            values: (0..len).map(&mut at).collect(),
            start,
            period: self.period(),
        })
    }

    fn combine(
        &self,
        left: Sequence,
        right: Sequence,
        op: impl Fn(bool, bool) -> bool,
    ) -> Result<Sequence, OracleError> {
        self.sequence(left.start.max(right.start), |i| op(left.at(i), right.at(i)))
    }

    fn eval(&self, formula: &Formula) -> Result<Sequence, OracleError> {
        use Formula::*;
        match formula {
            False => self.sequence(self.loop_entry, |_| false),
            True => self.sequence(self.loop_entry, |_| true),
            Atom(id) => self.sequence(self.loop_entry, |i| {
                let cell = if i < self.cells.len() {
                    i
                } else {
                    self.loop_entry + (i - self.loop_entry) % self.period()
                };
                self.cells[cell].get(id).copied().unwrap_or(false)
            }),
            Not(p) => Ok(self.eval(p)?.negated()),
            And(p, q) => self.combine(self.eval(p)?, self.eval(q)?, |a, b| a && b),
            Or(p, q) => self.combine(self.eval(p)?, self.eval(q)?, |a, b| a || b),
            Implies(p, q) => self.combine(self.eval(p)?, self.eval(q)?, |a, b| !a || b),
            Equivalent(p, q) => self.combine(self.eval(p)?, self.eval(q)?, |a, b| a == b),
            Future(range, p) => self.future(*range, self.eval(p)?, None),
            Globally(range, p) => Ok(self
                .future(*range, self.eval(p)?.negated(), None)?
                .negated()),
            Until(range, p, q) => self.future(*range, self.eval(q)?, Some(self.eval(p)?)),
            Release(range, p, q) => Ok(self
                .future(
                    *range,
                    self.eval(q)?.negated(),
                    Some(self.eval(p)?.negated()),
                )?
                .negated()),
            Once(range, p) => self.past(*range, self.eval(p)?, None),
            Historically(range, p) => {
                Ok(self.past(*range, self.eval(p)?.negated(), None)?.negated())
            }
            Since(range, p, q) => self.past(*range, self.eval(q)?, Some(self.eval(p)?)),
            Triggered(range, p, q) => Ok(self
                .past(
                    *range,
                    self.eval(q)?.negated(),
                    Some(self.eval(p)?.negated()),
                )?
                .negated()),
            StrongPrevious(p) => {
                self.past(Interval::Closed { start: 1, end: 1 }, self.eval(p)?, None)
            }
        }
    }

    fn future(
        &self,
        range: Interval,
        witness: Sequence,
        guard: Option<Sequence>,
    ) -> Result<Sequence, OracleError> {
        let (lower, upper) = self.check_range(range)?;
        let start = witness.start.max(guard.as_ref().map_or(0, |s| s.start));
        self.sequence(start, |time| {
            let ceiling = match upper {
                Some(end) => end,
                None => lower + start.saturating_sub(time) + self.period(),
            };
            let mut guard_holds = true;
            for offset in 0..=ceiling {
                if offset >= lower && guard_holds && witness.at(time + offset) {
                    return true;
                }
                if let Some(ref guard) = guard {
                    guard_holds &= guard.at(time + offset);
                    if !guard_holds {
                        return false;
                    }
                }
            }
            false
        })
    }

    fn past(
        &self,
        range: Interval,
        witness: Sequence,
        guard: Option<Sequence>,
    ) -> Result<Sequence, OracleError> {
        let (lower, upper) = self.check_range(range)?;
        let child_start = witness.start.max(guard.as_ref().map_or(0, |s| s.start));
        let offset = upper.unwrap_or(lower);
        let first = child_start
            .checked_add(offset)
            .and_then(|v| v.checked_add(self.period()))
            .ok_or(OracleError::ResourceIncomplete)?;
        let mut start = first;
        loop {
            let len = start
                .checked_add(self.period())
                .ok_or(OracleError::ResourceIncomplete)?;
            if len > self.limits.max_positions || start < self.period() {
                return Err(OracleError::ResourceIncomplete);
            }
            let values: Vec<bool> = (0..len)
                .map(|time| {
                    let ceiling = upper.unwrap_or(time).min(time);
                    let mut guard_holds = true;
                    for distance in 0..=ceiling {
                        if distance >= lower && guard_holds && witness.at(time - distance) {
                            return true;
                        }
                        if let Some(ref guard) = guard {
                            guard_holds &= guard.at(time - distance);
                            if !guard_holds {
                                return false;
                            }
                        }
                    }
                    false
                })
                .collect();
            let prior = start - self.period();
            if values[prior..start] == values[start..len] {
                return Ok(Sequence {
                    values,
                    start,
                    period: self.period(),
                });
            }
            start = start
                .checked_add(self.period())
                .ok_or(OracleError::ResourceIncomplete)?;
            if start > first.saturating_add(self.period() * 2) {
                return Err(OracleError::NoPeriodicFixedPoint);
            }
        }
    }

    fn check_range(&self, range: Interval) -> Result<(usize, Option<usize>), OracleError> {
        let (lower, upper) = range.bounds();
        if upper.is_some_and(|end| lower > end) {
            return Err(OracleError::InvertedInterval);
        }
        if lower > self.limits.max_offset || upper.is_some_and(|end| end > self.limits.max_offset) {
            return Err(OracleError::ResourceIncomplete);
        }
        Ok((lower, upper))
    }
}
