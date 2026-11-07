# Why an oracle

## What the crate is for

A production evaluator and its tests can share a mistake. If the tests compute their expected answers with the same code that produces the result, the two agree even when both are wrong. tl-oracle provides expected answers that come from a separate implementation of the semantics.

## Independence

The crate keeps these boundaries.

- It depends on tl-syntax for identities and documents only: `PropositionId`, `NodeId`, `SemanticProfile`, the infinite formula and lasso trace documents. It does not call a tl-syntax evaluator.
- It has no dependency on tl-mltl or tl-rewrite. A test in `tests/dependency_boundary.rs` runs `cargo tree` over the normal edges and fails when either crate appears.
- It defines its own formula tree and copies syntax into it with `from_syntax`.
- It evaluates each operator from its temporal clause. Duals such as `G`, `R`, `H` and `T` are defined through `F`, `U`, `O` and `S` and negation.
- It enumerates completions and builds whole sequences, and does not share work between completions.

## Differential checking

tl-mltl and tl-rewrite list tl-oracle as a git dev-dependency. A differential test builds one formula and one trace and compares the production result with the oracle's. tl-mltl passes infinite formula and lasso trace documents to `evaluate_documents`. tl-rewrite calls `evaluate_closed_trace_v1` and `evaluate_origin_complete` on finite words to check its rewrites.

The comparison code is in the consuming crate. The oracle does not call production code, so a result cannot agree with itself by construction.

The two profile functions follow the semantic profiles `mltl.closed-trace/v1` and `mltl.origin-complete-history/v1` defined in tl-syntax, so a test can ask for the answer under the profile that production implements.

## How the oracle is checked

The oracle has its own tests, which are listed here by file.

| File | What it checks |
| --- | --- |
| `tests/dependency_boundary.rs` | The normal dependency graph contains tl-syntax and excludes tl-mltl and tl-rewrite. |
| `tests/profiles.rs` | Non-zero offsets for U, R and S, the origin boundary for past operators, and the refusals of each profile. |
| `tests/reference.rs` | Fixed formulas and lassos, document-pair evaluation, the exhaustive syntax adapter, completions with fairness, and refusals under limits. |
| `tests/semantic_laws.rs` | 34 complete lassos with every loop entry through three cells. It checks duality, lasso unrolling, mixed past and future nesting over repeated laps, fairness filtering, monotonicity under partial information, bad-prefix refutation and seeded wrong outcomes. |
| `tests/exhaustive_small_scope.rs` | One declared finite partition, described below. |

The finite partition has one proposition, every Boolean and temporal constructor at depth one, six closed intervals with `0 <= a <= b <= 2` plus `[0,)`, and every complete word of length one through three at each valid position. Its cardinality is 12,954 cases, computed from the declaration. Of these, 11,322 are compared against a separate finite reference and 1,632 are refusals because of the open interval. The partition is complete at depth one and is not the full depth-three target.

`PopulationLedger` counts the cases. It records one visit for each case identifier from 0 to the declared count, and it reports an out-of-domain identifier, a duplicate visit, a mismatch with the reference and an incomplete partition. After a failure, `finish` returns that first failure even when later cases pass. The ledger counts visits and compares results. It cannot show that the caller's reference is independent.
