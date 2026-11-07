# SPDX-License-Identifier: MIT
# Copyright (C) 2026 Agent-IX
"""tl-oracle documentation figures, rendered by the docs-figures skill.

Regenerate from the repository root with the agent-ix/dev-tools skill:

    python3 -m pip install fonttools
    python3 <dev-tools>/skills/docs-figures/scripts/render.py docs/images/figures.py

Add --check to confirm the committed SVGs are current. Counts are read from
this repository: the Formula and OracleError enums and the Limits defaults in
src/lib.rs.

The block between the two SHARED markers is identical in every TL repository
(tl-syntax, tl-parse, tl-rewrite, tl-mltl, tl-oracle, quire-mltl). Copy it
verbatim and call tl_stack(here="<crate>") and stack_pipeline(here="<crate>").
"""

import re
from pathlib import Path

from docs_figures import FONTS, SMALL, TITLE, Figure  # noqa: F401

STYLE = "ix-docs"

ROOT = Path(__file__).resolve().parents[2]

# One role per kind of thing, fixed across the TL and Quire repositories.
ROLES = [
    ("code", "authored source", "box"),
    ("logic", "processing step", "box"),
    ("model", "checked document / view", "box"),
    ("evidence", "outcome", "box"),
    ("ink", "input", "box"),
]


# ==== SHARED: copy verbatim into sibling figures.py (begin) ===================

TL_CRATES = [
    # (column, row, name, one-line role, dependency note)
    (0, 1, "tl-parse", "text to formula graph"),
    (1, 1, "tl-oracle", "independent reference"),
    (2, 1, "tl-mltl", "evaluates traces"),
    (3, 1, "quire-mltl", "the bridge to Quire"),
    (2, 2, "tl-rewrite", "equivalent rewrites"),
    (3, 2, "quire-observation", "Quire observations"),
]


def tl_stack(here="tl-syntax"):
    """The TL crate family as a dependency graph; `here` is drawn highlighted."""
    f = Figure(
        "tl-stack",
        980,
        356,
        "The TL crates. tl-syntax defines the formula graph, profiles and signal catalog, and "
        "tl-parse, tl-oracle, tl-mltl, tl-rewrite and quire-mltl all depend on it. tl-rewrite "
        "depends on tl-mltl. quire-mltl depends on tl-mltl and quire-observation, and is the "
        "single bridge to Quire. Dashed lines are development-only: tl-mltl and tl-rewrite "
        "test against tl-oracle, and tl-rewrite tests against tl-parse.",
    )
    w, h, step = 192, 56, 262
    rows = {0: 24, 1: 140, 2: 256}
    f.text(3 * step, 244, "QUIRE", "muted", "mono", SMALL)
    f.box(0, rows[0], 980, h, "logic" if here == "tl-syntax" else "ink", "tl-syntax",
          "formula graph · semantic profiles · signal catalog · strict documents")
    where = {}
    for col, row, name, sub in TL_CRATES:
        x, y = col * step, rows[row]
        where[name] = (x, y)
        role = "logic" if name == here else "ink"
        f.box(x, y, w, h, role, name, sub, dashed=(name == "quire-observation"))
    for name in ("tl-parse", "tl-oracle", "tl-mltl", "quire-mltl"):
        x, y = where[name]
        f.line([(x + w / 2, rows[0] + h), (x + w / 2, y - 3)], "model")
    f.note(where["tl-parse"][0] + w / 2 + 10, 104, "types", "model")
    mx, my = where["tl-mltl"]
    rx, ry = where["tl-rewrite"]
    f.line([(mx + w / 2, my + h), (rx + w / 2, ry - 3)], "model")
    f.note(mx + w / 2 + 10, my + h + 30, "documents", "model")
    qx, qy = where["quire-mltl"]
    f.line([(mx + w, my + h / 2), (qx - 3, qy + h / 2)], "model")
    f.note(mx + w + 8, my + h / 2 - 8, "results", "model")
    ox, oy = where["quire-observation"]
    f.line([(ox + w / 2, oy - 3), (ox + w / 2, qy + h)], "model")
    f.note(ox + w / 2 + 10, qy + h + 30, "observations", "model")
    # development-only edges: tl-oracle and tl-parse feed tests of tl-mltl and tl-rewrite
    px, py = where["tl-parse"]
    zx, zy = where["tl-oracle"]
    f.line([(zx + w, zy + h / 2), (mx - 3, my + h / 2)], "model", dashed=True)
    f.line([(zx + w / 2, zy + h), (zx + w / 2, ry + 18), (rx - 3, ry + 18)], "model", dashed=True)
    f.line([(px + w / 2, py + h), (px + w / 2, ry + 40), (rx - 3, ry + 40)], "model", dashed=True)
    f.legend(0, 350, [("logic", "this repository", "box"), ("ink", "other crate", "box"),
                      ("model", "depends on", "solid"), ("model", "development only", "dash")])
    return f


PIPELINE_STAGES = [
    ("MLTL text", "your formula", "code", None),
    ("tl-parse", "parse", "logic", "tl-parse"),
    ("tl-rewrite", "rewrite", "logic", "tl-rewrite"),
    ("tl-mltl", "evaluate", "logic", "tl-mltl"),
    ("tl-oracle", "tests only", "logic", "tl-oracle"),
    ("quire-mltl", "map to Quire", "logic", "quire-mltl"),
]


def stack_pipeline(here="tl-syntax"):
    """End-to-end flow: text, parse, rewrite, evaluate with an oracle cross-check, Quire view."""
    f = Figure(
        "pipeline",
        980,
        392,
        "A formula moves from text to a Quire outcome. tl-parse turns MLTL text into a formula "
        "document, tl-rewrite returns a rewritten document, and tl-mltl evaluates it against a "
        "trace. In tests, tl-oracle evaluates the same formula and trace independently and the "
        "verdicts are compared. quire-mltl maps the result to a Boolean Value or a typed "
        "NonValue reason. All documents use the vocabulary of tl-syntax.",
    )
    w, h = 140, 56
    step = (980 - w) / (len(PIPELINE_STAGES) - 1)
    cx = [round(i * step + w / 2) for i in range(len(PIPELINE_STAGES))]
    for i, (name, sub, role, crate) in enumerate(PIPELINE_STAGES):
        if crate is not None:
            role = "logic" if crate == here else "ink"
        f.box(cx[i] - w // 2, 0, w, h, role, name, sub)
        f.line([(cx[i], h + 2), (cx[i], 312)], "faint", dashed=True, arrow=False, weight=1.0)
    msgs = [
        (0, 1, 96, "MLTL text", "code", False),
        (1, 2, 140, "formula document", "model", False),
        (2, 3, 184, "rewritten formula", "model", False),
        (3, 4, 228, "formula + trace", "model", True),
        (4, 3, 262, "reference verdict", "evidence", True),
        (3, 5, 306, "result", "model", False),
    ]
    for a, b, y, label, role, dev in msgs:
        sign = 1 if b > a else -1
        f.line([(cx[a] + 3 * sign, y), (cx[b] - 3 * sign, y)], role, dashed=dev,
               weight=2.4 if not dev else 1.5)
        left = min(cx[a], cx[b]) + 10
        f.note(left, y - 8, label, role)
    f.box(980 - 190, 332, 190, 50, "evidence", "Value or NonValue", "true / false or reason")
    f.line([(cx[5], 312), (cx[5], 329)], "evidence")
    f.box(0, 332, 520, 50, "logic" if here == "tl-syntax" else "ink", "tl-syntax",
          "the formula, profile, signal and document types every step uses")
    return f


# ==== SHARED (end) ============================================================


# ---- repository data ---------------------------------------------------------


def lib_source():
    return (ROOT / "src/lib.rs").read_text(encoding="utf-8")


def enum_variants(name):
    """Variant names of `pub enum <name>` in src/lib.rs, in source order."""
    text = lib_source()
    start = text.index(f"pub enum {name} {{")
    body = text[start : text.index("\n}\n", start)]
    body = "\n".join(line for line in body.splitlines() if not line.strip().startswith("//"))
    return re.findall(r"^    ([A-Z]\w*)\b", body, flags=re.M)


def limit_default(field):
    """Default value of a `Limits` field, read from the `Default` impl."""
    text = lib_source()
    return int(re.search(rf"{field}:\s*([\d_]+),", text[text.index("impl Default for Limits") :]).group(1).replace("_", ""))


# ---- figures -----------------------------------------------------------------


def hero():
    """README hero: tl-mltl and tl-oracle evaluate the same formula and trace; the results are compared."""
    n_formula = len(enum_variants("Formula"))
    n_error = len(enum_variants("OracleError"))
    n_evidence = len(enum_variants("Evidence"))
    max_completions = limit_default("max_completions")
    f = Figure(
        "hero",
        980,
        392,
        "A formula and a lasso trace go to two independent evaluators. tl-mltl, the production "
        "evaluator, returns a verdict. tl-oracle converts the formula to its own tree of "
        f"{n_formula} node kinds, enumerates the completions of missing and conflicting "
        f"evidence (at most {max_completions:,} by default), filters them by fairness, and returns an Outcome "
        "or one of its "
        f"{n_error} OracleError refusals. A differential test in tl-mltl compares the two results "
        "and reports whether they agree.",
    )
    f.text(0, 148, "INPUT", "muted", "mono", SMALL)
    f.box(0, 160, 232, 78, "code", "formula + trace", "claim and fairness roots", f"lasso with {n_evidence} evidence values")
    f.text(260, 12, "PRODUCTION LANE", "muted", "mono", SMALL)
    f.box(260, 24, 200, 62, "ink", "tl-mltl", "production evaluator")
    f.box(520, 24, 170, 62, "model", "verdict", "production result")
    f.line([(116, 157), (116, 55), (257, 55)], "code")
    f.line([(460, 55), (517, 55)], "ink", weight=2.4)
    f.text(260, 252, "ORACLE LANE", "muted", "mono", SMALL)
    f.box(260, 264, 150, 62, "logic", "from_syntax", "document to tree")
    f.box(440, 264, 200, 62, "logic", "evaluate", "completions · fairness")
    f.box(670, 264, 150, 62, "evidence", "Outcome", "verdict + counts")
    f.line([(116, 241), (116, 295), (257, 295)], "code")
    f.line([(410, 295), (437, 295)], "logic", weight=2.4)
    f.line([(640, 295), (667, 295)], "logic", weight=2.4)
    f.box(840, 130, 140, 78, "logic", "compare", "differential test", "agree or differ")
    f.line([(690, 55), (910, 55), (910, 127)], "model")
    f.note(700, 49, "verdict", "model")
    f.line([(820, 295), (910, 295), (910, 209)], "evidence")
    f.note(826, 289, "Outcome", "evidence")
    f.legend(0, 372, [ROLES[0], ROLES[1], ROLES[2], ROLES[3], ROLES[4]])
    return f


def tl_stack_here():
    return tl_stack(here="tl-oracle")


def pipeline_here():
    return stack_pipeline(here="tl-oracle")


FIGURES = [hero, tl_stack_here, pipeline_here]
