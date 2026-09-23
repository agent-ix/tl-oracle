# tl-oracle

Independent, development-only reference semantics for the Stage 1 TL crates.
It uses public `tl-syntax` identities and has no dependency on `tl-mltl` or
`tl-rewrite`. It is a public MIT-licensed development tool, but is not
published as a crate.

The oracle evaluates each formula directly from the temporal clauses over an
ultimately periodic word. Unknown observations are enumerated as common Boolean
completions of each lasso cell. A configured resource limit returns an error;
it never turns incomplete exploration into a proof.

For bounded words, `evaluate_closed_trace_v1` and `evaluate_origin_complete`
match the two owner profiles explicitly. The former begins bounded U/R guards
at the interval lower endpoint and pads absent future atoms with false. The
latter evaluates past intervals across the origin with false pre-origin atoms.
The older `evaluate_finite` keeps its origin-limited, offset-zero guard
reference clauses for the independent finite partition and law tests; it is
not an owner-profile comparison API.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` locally. The `TC-175`, `TC-176`, `TC-193`, and `TC-194` tests are
the initial independent-oracle checks.

`tests/exhaustive_small_scope.rs` completes one declared finite partition:
one proposition, all Boolean and temporal operator constructors at depth one,
six closed intervals with `0 <= a <= b <= 2` plus `[0,)`, and every complete
word of length one through three at each valid position. Its independently
calculated cardinality is 12,954 cases: 11,322 admitted comparisons against a
separate finite reference and 1,632 explicit open-interval refusals. The
`PopulationLedger` detects omissions, duplicates, out-of-domain IDs and wrong
answers; a recorded failure cannot be cleared by a later successful visit.
This is a completed finite partition, not the full depth-three V1 target.

`tests/semantic_laws.rs` enumerates 34 complete lassos with every loop entry
through three materialized cells. It checks duality, lasso unrolling, mixed
past/future nesting over repeated laps, fairness filtering, partial-information
monotonicity, bad-prefix refutation and seeded wrong outcomes. Production
comparison remains the responsibility of the consuming `tl-mltl` test lane;
the oracle itself has no production dependency.
