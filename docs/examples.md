# Examples

Every example here compiles and passes against the crate. They use `tl_syntax::PropositionId` and `std::collections::BTreeMap`, so add `tl-syntax` next to `tl-oracle` in `Cargo.toml`.

## Evaluate a lasso with missing evidence

```rust
use std::collections::BTreeMap;
use tl_oracle::{evaluate, Evidence, Formula, Lasso, Limits, Uncertainty, Verdict};
use tl_syntax::PropositionId;

let p = PropositionId(0);
let cell = BTreeMap::from([(p, Evidence::Missing)]);
let lasso = Lasso::new(vec![], vec![cell]).unwrap();

// p & !p is false under both completions of p.
let claim = Formula::And(
    Box::new(Formula::Atom(p)),
    Box::new(Formula::Not(Box::new(Formula::Atom(p)))),
);
let outcome = evaluate(&claim, &[], &lasso, 0, Limits::default()).unwrap();
assert_eq!(outcome.verdict, Verdict::Refuted);
assert_eq!(outcome.fair_completions, 2);
assert_eq!(outcome.uncertainty, Uncertainty::Missing);

// p alone depends on the completion.
let outcome = evaluate(&Formula::Atom(p), &[], &lasso, 0, Limits::default()).unwrap();
assert_eq!(outcome.verdict, Verdict::Inconclusive);
assert!(outcome.possible_true && outcome.possible_false);
```

This is the partial-completion case in `tests/reference.rs`. A completion is chosen once for the cell, so `p & !p` cannot be true.

## Filter completions by fairness

```rust
use std::collections::BTreeMap;
use tl_oracle::{evaluate, Evidence, Formula, Lasso, Limits, Verdict};
use tl_syntax::PropositionId;

let p = PropositionId(0);
let lasso = Lasso::new(vec![], vec![BTreeMap::from([(p, Evidence::Missing)])]).unwrap();

// Requiring p infinitely often removes the completion where p is false.
let outcome = evaluate(&Formula::Atom(p), &[Formula::Atom(p)], &lasso, 3, Limits::default()).unwrap();
assert_eq!(outcome.verdict, Verdict::Proved);
assert_eq!(outcome.fair_completions, 1);
```

## Compare the finite profiles

```rust
use std::collections::BTreeMap;
use tl_oracle::{evaluate_closed_trace_v1, evaluate_finite, Formula, Interval, Limits, OracleError};
use tl_syntax::PropositionId;

let p = PropositionId(0);
let word = [BTreeMap::from([(p, false)]), BTreeMap::from([(p, true)])];
let range = Interval::Closed { start: 1, end: 1 };

let until = Formula::Until(range, Box::new(Formula::Atom(p)), Box::new(Formula::Atom(p)));
// The closed-trace guard starts at offset 1, so the false cell at offset 0 is not read.
assert_eq!(evaluate_closed_trace_v1(&until, &word, 0, Limits::default()), Ok(true));
// The original finite clauses start the guard at offset 0.
assert_eq!(evaluate_finite(&until, &word, 0, Limits::default()), Ok(false));

// A past operator is outside the closed-trace profile.
let once = Formula::Once(range, Box::new(Formula::Atom(p)));
assert_eq!(
    evaluate_closed_trace_v1(&once, &word, 0, Limits::default()),
    Err(OracleError::ClosedTracePastUnsupported)
);
```

This is the closed-trace case in `tests/profiles.rs`.

## Limits and refusals

```rust
use std::collections::BTreeMap;
use tl_oracle::{evaluate, Evidence, Formula, Lasso, Limits, OracleError};
use tl_syntax::PropositionId;

let p = PropositionId(0);

// One unknown needs two completions; a limit of one returns an error.
let unknown = Lasso::new(vec![], vec![BTreeMap::from([(p, Evidence::Missing)])]).unwrap();
let limits = Limits { max_completions: 1, ..Limits::default() };
assert_eq!(
    evaluate(&Formula::Atom(p), &[], &unknown, 0, limits),
    Err(OracleError::ResourceIncomplete)
);

// A premise that no completion satisfies leaves nothing to evaluate.
let false_word = Lasso::new(vec![], vec![BTreeMap::from([(p, Evidence::False)])]).unwrap();
assert_eq!(
    evaluate(&Formula::True, &[Formula::Atom(p)], &false_word, 0, Limits::default()),
    Err(OracleError::EmptyFairAdmission)
);
```

This follows the refusal test in `tests/reference.rs`.

## Evaluate a document pair

```rust
use tl_oracle::{evaluate_documents, Limits, Uncertainty, Verdict};
use tl_syntax::{
    InfiniteClock, InfiniteFormulaDocument, InfiniteNode, InfiniteNodeKind, LassoTraceDocument,
    NodeId, PartialValuation, PartialValue, PropositionId, SemanticProfile, TraceObservation,
    ValuationEntry,
};

let formula = InfiniteFormulaDocument::new(
    SemanticProfile::InfiniteTraceV1,
    InfiniteClock::EventPosition,
    NodeId(0),
    vec![InfiniteNode::new(InfiniteNodeKind::Proposition { proposition: PropositionId(0) })],
)
.unwrap();

let map = "map-1".to_owned();
let valuation = PartialValuation::new(
    map.clone(),
    &[PropositionId(0)],
    vec![ValuationEntry { proposition: PropositionId(0), value: PartialValue::Conflicting }],
)
.unwrap();
let trace = LassoTraceDocument::new(
    SemanticProfile::InfiniteTraceV1,
    InfiniteClock::EventPosition,
    map,
    vec![PropositionId(0)],
    vec![],
    vec![TraceObservation { position: 0, valuation }],
)
.unwrap();

let outcome = evaluate_documents(&formula, &trace, NodeId(0), &[], 4, Limits::default()).unwrap();
assert_eq!(outcome.verdict, Verdict::Inconclusive);
assert_eq!(outcome.uncertainty, Uncertainty::Conflicting);
```

This is the document-pair case in `tests/reference.rs`. `evaluate_documents` checks that both documents use `mltl.infinite-trace/v1` with the same clock and that the trace's proposition map admits every proposition the formula uses, then evaluates.

## Count a finite partition

```rust
use tl_oracle::population::{CaseResult, PopulationError, PopulationLedger};

let mut ledger = PopulationLedger::new(2).unwrap();
ledger.record(0, CaseResult::Admitted(true), CaseResult::Admitted(true)).unwrap();
ledger.record(1, CaseResult::<bool>::Refused, CaseResult::Refused).unwrap();
let summary = ledger.finish().unwrap();
assert_eq!((summary.declared, summary.admitted, summary.refused), (2, 1, 1));

// A partition with an unvisited case does not finish.
let mut partial = PopulationLedger::new(2).unwrap();
partial.record(0, CaseResult::Admitted(true), CaseResult::Admitted(true)).unwrap();
assert_eq!(partial.finish(), Err(PopulationError::Incomplete { declared: 2, visited: 1 }));
```

`tests/exhaustive_small_scope.rs` uses the same ledger for a partition of 12,954 cases.
