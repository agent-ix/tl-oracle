# tl-oracle documentation

tl-oracle is an independent reference semantics for MLTL formulas over finite words and lassos. It is a development-only crate of the TL family. This directory documents what the crate computes and how to call it. The [README](../README.md) has the overview and the smallest example.

| Page | What it covers |
| --- | --- |
| [how-it-works.md](how-it-works.md) | The oracle's formula tree, how a lasso is evaluated, the bounded profile evaluators, and where the crate sits in the pipeline, and its step in a worked debugging session. |
| [why-an-oracle.md](why-an-oracle.md) | Why the crate stays independent of tl-mltl, how it is used for differential checking, and how its own tests check it. |
| [evidence.md](evidence.md) | Lasso traces, the five kinds of evidence, completions, fairness, verdicts and the outcome record. |
| [examples.md](examples.md) | Runnable examples: profiles, refusals, fairness, documents and the population ledger. |
| [api-reference.md](api-reference.md) | Public functions, types and errors. |
