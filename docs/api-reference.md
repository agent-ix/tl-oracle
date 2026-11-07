# API reference

Functions and types are at the crate root. The counting helpers are in the `population` module. Every function returns `Result<_, OracleError>`.

## Evaluation

| Function | Signature summary | Purpose |
| --- | --- | --- |
| `evaluate` | `(&Formula, &[Formula], &Lasso, usize, Limits) -> Result<Outcome, OracleError>` | Claim, fairness premises, lasso, position, limits. Enumerates completions. |
| `evaluate_syntax` | `(InfiniteFormula, NodeId, &[NodeId], &Lasso, usize, Limits) -> Result<Outcome, OracleError>` | Converts the claim and fairness roots of one tl-syntax graph, then calls `evaluate`. |
| `evaluate_documents` | `(&InfiniteFormulaDocument, &LassoTraceDocument, NodeId, &[NodeId], usize, Limits) -> Result<Outcome, OracleError>` | Checks profile, clock, roots and proposition map, then evaluates. |
| `evaluate_closed_trace_v1` | `(&Formula, &[BTreeMap<PropositionId, bool>], usize, Limits) -> Result<bool, OracleError>` | Finite word under `mltl.closed-trace/v1`. |
| `evaluate_origin_complete` | `(&Formula, &[BTreeMap<PropositionId, bool>], usize, Limits) -> Result<bool, OracleError>` | Finite history under `mltl.origin-complete-history/v1`. |
| `evaluate_finite` | `(&Formula, &[BTreeMap<PropositionId, bool>], usize, Limits) -> Result<bool, OracleError>` | Finite word under the original clauses. |
| `from_syntax` | `(InfiniteFormula, NodeId) -> Result<Formula, OracleError>` | Copies one root of a syntax graph into an oracle `Formula`. |

## Types

| Item | Purpose |
| --- | --- |
| `Formula` | 17 variants: `False`, `True`, `Atom(PropositionId)`, `Not`, `And`, `Or`, `Implies`, `Equivalent`, `Future`, `Globally`, `Until`, `Release`, `Once`, `Historically`, `Since`, `Triggered`, `StrongPrevious`. Children are `Box<Formula>`. Temporal variants take an `Interval` first. |
| `Interval` | `Closed { start: usize, end: usize }` or `Unbounded { start: usize }`. |
| `Evidence` | `False`, `True`, `Missing`, `Conflicting`, `Impossible`. |
| `Lasso` | `new(prefix, loop_cells)`, `loop_entry()`, and `From<&LassoTraceDocument>`. Cells are `BTreeMap<PropositionId, Evidence>`. |
| `Limits` | Fields `max_depth`, `max_offset`, `max_positions`, `max_completions`. `Default` gives 64, 4,096, 65,536 and 65,536. |
| `Verdict` | `Proved`, `Refuted`, `Inconclusive`. |
| `Uncertainty` | `None`, `Missing`, `Conflicting`, `Mixed`. |
| `Outcome` | Fields `verdict`, `possible_true`, `possible_false`, `fair_completions`, `uncertainty`. |
| `OracleError` | 14 refusal and limit variants, listed in [evidence.md](evidence.md#refusals). |

`Interval`, `Evidence`, `Verdict`, `Uncertainty`, `Outcome` and `OracleError` are `Copy`, and with `Formula` they implement `Clone`, `Debug`, `Eq` and `PartialEq`. `Lasso` implements `Clone` and `Debug`. `Limits` implements `Clone`, `Copy`, `Debug` and `Default`. `OracleError` implements `std::error::Error` through `thiserror`.

## population

| Item | Purpose |
| --- | --- |
| `PopulationLedger` | `new(declared)`, `record(case_id, reference, observed)`, `finish()`. |
| `CaseResult<V>` | `Admitted(V)` or `Refused`. |
| `PopulationSummary` | Fields `declared`, `admitted`, `refused`. |
| `PopulationError` | `Empty`, `OutOfDomain`, `Duplicate`, `Mismatch`, `Incomplete`. |

`record` compares the reference result to the observed result and fails on an identifier outside `0..declared`, on a repeated identifier and on a mismatch. `finish` fails when any case was not visited, and it returns the first recorded failure even when later cases succeeded.

## Dependencies

| Crate | Use |
| --- | --- |
| `tl-syntax` | `PropositionId`, `NodeId`, `SemanticProfile`, the infinite formula and lasso trace documents. Built with the `alloc` feature. |
| `thiserror` | Error derives. |
