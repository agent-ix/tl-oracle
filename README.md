# tl-oracle

Independent, development-only reference semantics for the Stage 1 TL crates.
It uses public `tl-syntax` identities and has no dependency on `tl-mltl` or
`tl-rewrite`. It is not a production package and is not published.

The oracle evaluates each formula directly from the temporal clauses over an
ultimately periodic word. Unknown observations are enumerated as common Boolean
completions of each lasso cell. A configured resource limit returns an error;
it never turns incomplete exploration into a proof.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` locally. The `TC-175`, `TC-176`, `TC-193`, and `TC-194` tests are
the initial independent-oracle checks.
