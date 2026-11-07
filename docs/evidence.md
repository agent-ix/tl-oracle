# Evidence, verdicts and outcomes

## Lasso traces

A `Lasso` is a prefix of cells followed by a nonempty loop that repeats forever. `Lasso::new(prefix, loop_cells)` returns `EmptyLoop` when the loop has no cell. `loop_entry()` is the index of the first loop cell, which equals the prefix length. A position that lies beyond the stored cells reads the loop cell at the same place in the cycle.

A `LassoTraceDocument` from tl-syntax converts with `Lasso::from(&document)`. Its `PartialValue` entries become `Evidence`, one cell for each prefix and loop observation.

## Evidence

Each cell maps a proposition to one `Evidence` value.

| Evidence | Meaning | Completions |
| --- | --- | --- |
| `False` | The proposition is known false. | One. |
| `True` | The proposition is known true. | One. |
| `Missing` | No observation has arrived. A proposition that the cell does not list is also missing. | Two. |
| `Conflicting` | Both true and false observations exist. | Two. |
| `Impossible` | The possibility set is empty and the input is inconsistent. | None. `evaluate` returns `InconsistentInput`. |

`Missing` and `Conflicting` enumerate the same two Boolean values. They differ in the reason they report.

## Completions and fairness

A completion gives each unknown one Boolean value. The oracle enumerates all `2^n` completions of the `n` unknowns. A value fixed for a cell stays fixed for every reference to that proposition and every lap of the loop, so `p & !p` stays false.

Fairness premises are formulas. A completion is fair when each premise is true at some position within the loop, which means it is true infinitely often on the repeating part. Only fair completions count toward the result.

## Verdict

The `Verdict` describes the claim at one position, over the fair completions.

| Verdict | Meaning |
| --- | --- |
| `Proved` | Every fair completion makes the claim true. |
| `Refuted` | Every fair completion makes the claim false. |
| `Inconclusive` | Fair completions disagree. |

The verdict applies to the trace and position that were given. It makes no claim about a model.

## Outcome

`Outcome` holds the whole result.

| Field | Meaning |
| --- | --- |
| `verdict` | The `Verdict`. |
| `possible_true` | At least one fair completion makes the claim true. |
| `possible_false` | At least one fair completion makes the claim false. |
| `fair_completions` | The number of fair completions. |
| `uncertainty` | `None`, `Missing`, `Conflicting` or `Mixed`. It records the kind of open evidence that was present, even when the formula settles. |

For example, `p` on a cell where `p` is `True` and an unrelated proposition is `Missing` is `Proved` with two fair completions and `Uncertainty::Missing`.

## Refusals

`OracleError` reports a refusal or an exhausted limit. It never carries a truth value.

| Error | Cause |
| --- | --- |
| `EmptyLoop` | A lasso without a loop cell. |
| `InvertedInterval` | A closed interval with `start` greater than `end`. |
| `InconsistentInput` | A cell with `Impossible` evidence. |
| `EmptyFairAdmission` | No completion satisfies every fairness premise. |
| `ResourceIncomplete` | A limit was exceeded, or syntax conversion passed its fixed depth or size. |
| `NoPeriodicFixedPoint` | A past operator did not reach a repeating pattern within the bound. |
| `GraphRootAbsent` | The requested root is not in the syntax graph. |
| `FinitePositionAbsent` | The position is outside the finite word. |
| `FinitePropositionMissing` | `evaluate_finite` found a referenced proposition missing from a cell. |
| `ClosedTracePastUnsupported` | A past operator reached `evaluate_closed_trace_v1`. |
| `OriginFutureUnsupported` | A future operator reached `evaluate_origin_complete`. |
| `UnboundedFiniteInterval` | An open interval reached a finite-word evaluator. |
| `IdentityMismatch` | `evaluate_documents` received documents with different profiles or clocks, or a profile other than `mltl.infinite-trace/v1`. |
| `ForeignProposition` | The formula uses a proposition that the trace's proposition map lacks. |
