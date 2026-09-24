//! FR-053/TC-193–194: declared, distinct V11 lasso population for independent provider comparison.
//!
//! This is a completed small partition, not the full unbounded V11 target.
//! Formula and trace construction uses public syntax types only; no production
//! evaluator or parser is imported. The caller compares each case with the
//! oracle and the production provider, recording pass, refusal, or incomplete.

use tl_syntax::{
    InfiniteClock, InfiniteFormulaDocument, InfiniteNode, InfiniteNodeKind as Kind, Interval,
    LassoTraceDocument, NodeId, PartialValuation, PartialValue, PropositionId, SemanticProfile,
    TemporalInterval, TraceObservation, UnboundedInterval, ValuationEntry,
};

/// Formula shapes in the completed partition: leaves, Boolean operators,
/// every infinite-profile temporal operator, and four mixed nestings.
pub const FORMULA_COUNT: usize = 30;
/// Distinct complete two-proposition lassos through three materialized cells.
pub const COMPLETE_WORD_COUNT: usize = 228;
/// Distinct one-unknown lassos through two materialized cells.
pub const SINGLE_UNKNOWN_WORD_COUNT: usize = 136;
/// Distinct two-unknown mixed Missing/Conflicting lassos of length two.
pub const MIXED_WORD_COUNT: usize = 8;
/// Total distinct lasso words in this partition.
pub const WORD_COUNT: usize = COMPLETE_WORD_COUNT + SINGLE_UNKNOWN_WORD_COUNT + MIXED_WORD_COUNT;
/// No premise, p-fair, or q-fair.
pub const FAIRNESS_COUNT: usize = 3;
/// Origin, first lap, and two later-loop positions.
pub const ANCHORS: [u64; 4] = [0, 1, 3, 6];
/// Independently declared case count: 30 × 372 × 3 × 4.
pub const DECLARED_CASES: u64 =
    (FORMULA_COUNT * WORD_COUNT * FAIRNESS_COUNT * ANCHORS.len()) as u64;
/// Independently counted empty-fair word/mode pairs: 106 p, 104 q.
pub const EMPTY_FAIR_WORD_MODES: usize = 210;
/// Refused cases from empty fair admission across all formulas and anchors.
pub const EMPTY_FAIR_CASES: u64 = (EMPTY_FAIR_WORD_MODES * FORMULA_COUNT * ANCHORS.len()) as u64;
/// Cases with at least one fair Boolean completion.
pub const ADMITTED_CASES: u64 = DECLARED_CASES - EMPTY_FAIR_CASES;

/// One named formula graph. Proposition nodes p and q are stable at 0 and 1.
#[derive(Clone, Debug)]
pub struct V11Formula {
    /// Stable partition label, useful in a counterexample.
    pub name: String,
    /// Strict owner graph and exact profile/clock identity.
    pub graph: InfiniteFormulaDocument,
}

/// Which uncertainty sources occur in a generated word.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    /// Every proposition observation is known true or false.
    Complete,
    /// Exactly one observation is missing.
    Missing,
    /// Exactly one observation is conflicting.
    Conflicting,
    /// One missing and one conflicting observation coexist.
    Mixed,
}

/// One distinct complete or partial lasso.
#[derive(Clone, Debug)]
pub struct V11Word {
    /// Stable ordinal in the declared word axis.
    pub index: usize,
    /// The owner's strict lasso document.
    pub trace: LassoTraceDocument,
    /// Independent uncertainty class of the generated cells.
    pub evidence_class: EvidenceClass,
    /// Number of nonrepeating prefix cells.
    pub prefix_len: usize,
    cells: Vec<[PartialValue; 2]>,
}

impl V11Word {
    /// Returns materialized cells in p,q order for independent count checks.
    pub fn cells(&self) -> &[[PartialValue; 2]] {
        &self.cells
    }

    /// Counts admitted Boolean completions without invoking either evaluator.
    ///
    /// Each Missing or Conflicting entry contributes one independent Boolean
    /// choice. An atomic fairness premise admits a completion iff its atom is
    /// true at least once in the loop. A false premise therefore admits zero.
    pub fn expected_fair_completions(&self, fairness: Fairness) -> u64 {
        let unknown = self
            .cells
            .iter()
            .flat_map(|cell| cell.iter())
            .filter(|value| matches!(value, PartialValue::Missing | PartialValue::Conflicting))
            .count();
        let total = 1_u64 << unknown;
        let Some(proposition) = fairness.proposition_index() else {
            return total;
        };
        let loop_values = self.cells[self.prefix_len..]
            .iter()
            .map(|cell| cell[proposition]);
        if loop_values.clone().any(|value| value == PartialValue::True) {
            return total;
        }
        let unknown_loop = loop_values
            .filter(|value| matches!(value, PartialValue::Missing | PartialValue::Conflicting))
            .count();
        total - (1_u64 << (unknown - unknown_loop))
    }
}

/// Fairness mode for one exact graph: no premise, p, or q.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fairness {
    /// No fairness premise.
    None,
    /// Proposition p at graph node zero.
    P,
    /// Proposition q at graph node one.
    Q,
}

impl Fairness {
    /// Every fairness choice in stable index order.
    pub const ALL: [Self; FAIRNESS_COUNT] = [Self::None, Self::P, Self::Q];

    /// Same-graph premise roots, in their declared order.
    pub fn roots(self) -> &'static [NodeId] {
        match self {
            Self::None => &[],
            Self::P => &[NodeId(0)],
            Self::Q => &[NodeId(1)],
        }
    }

    fn proposition_index(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::P => Some(0),
            Self::Q => Some(1),
        }
    }
}

/// Full finite axes of the completed V11 partition.
#[derive(Debug)]
pub struct V11Population {
    formulas: Vec<V11Formula>,
    words: Vec<V11Word>,
}

impl Default for V11Population {
    fn default() -> Self {
        Self::new()
    }
}

impl V11Population {
    /// Constructs all finite axes and checks their fixed declared sizes.
    pub fn new() -> Self {
        let formulas = formulas();
        let words = words();
        assert_eq!(formulas.len(), FORMULA_COUNT, "V11 formula axis drift");
        assert_eq!(words.len(), WORD_COUNT, "V11 word axis drift");
        Self { formulas, words }
    }

    /// Named formula graphs in stable order.
    pub fn formulas(&self) -> &[V11Formula] {
        &self.formulas
    }

    /// Distinct lasso words in stable order.
    pub fn words(&self) -> &[V11Word] {
        &self.words
    }

    /// Deterministic contiguous case identifier, or absent for an invalid axis.
    pub fn case_id(
        &self,
        formula: usize,
        word: usize,
        fairness: usize,
        anchor: usize,
    ) -> Option<u64> {
        if formula >= self.formulas.len()
            || word >= self.words.len()
            || fairness >= FAIRNESS_COUNT
            || anchor >= ANCHORS.len()
        {
            return None;
        }
        let ordinal =
            (((formula * WORD_COUNT + word) * FAIRNESS_COUNT + fairness) * ANCHORS.len()) + anchor;
        u64::try_from(ordinal).ok()
    }
}

fn graph(name: impl Into<String>, mut nodes: Vec<InfiniteNode>, root: Kind) -> V11Formula {
    let root_id = NodeId(u32::try_from(nodes.len()).expect("finite V11 node count"));
    nodes.push(InfiniteNode::new(root));
    V11Formula {
        name: name.into(),
        graph: InfiniteFormulaDocument::new(
            SemanticProfile::InfiniteTraceV1,
            InfiniteClock::EventPosition,
            root_id,
            nodes,
        )
        .expect("fixed V11 formula is owner valid"),
    }
}

fn atom_graph(name: impl Into<String>, root: NodeId) -> V11Formula {
    V11Formula {
        name: name.into(),
        graph: InfiniteFormulaDocument::new(
            SemanticProfile::InfiniteTraceV1,
            InfiniteClock::EventPosition,
            root,
            base(),
        )
        .expect("fixed V11 atom graph is owner valid"),
    }
}

fn base() -> Vec<InfiniteNode> {
    vec![
        InfiniteNode::new(Kind::Proposition {
            proposition: PropositionId(0),
        }),
        InfiniteNode::new(Kind::Proposition {
            proposition: PropositionId(1),
        }),
    ]
}

fn formulas() -> Vec<V11Formula> {
    let p = NodeId(0);
    let q = NodeId(1);
    let mut cases = Vec::with_capacity(FORMULA_COUNT);
    cases.push(graph("false", base(), Kind::False));
    cases.push(graph("true", base(), Kind::True));
    cases.push(atom_graph("p", p));
    cases.push(atom_graph("q", q));
    for (name, kind) in [
        ("not-p", Kind::Not { operand: p }),
        ("and", Kind::And { left: p, right: q }),
        ("or", Kind::Or { left: p, right: q }),
        ("implies", Kind::Implies { left: p, right: q }),
        ("equivalent", Kind::Equivalent { left: p, right: q }),
    ] {
        cases.push(graph(name, base(), kind));
    }
    let intervals = [
        (
            "closed-1-2",
            TemporalInterval::Closed(Interval::new(1, 2).expect("valid V11 interval")),
        ),
        (
            "unbounded-0",
            TemporalInterval::Unbounded(UnboundedInterval::new(0)),
        ),
    ];
    for (suffix, interval) in intervals {
        for (name, kind) in [
            (
                "future",
                Kind::Future {
                    interval,
                    operand: p,
                },
            ),
            (
                "globally",
                Kind::Globally {
                    interval,
                    operand: p,
                },
            ),
            (
                "once",
                Kind::Once {
                    interval,
                    operand: p,
                },
            ),
            (
                "historically",
                Kind::Historically {
                    interval,
                    operand: p,
                },
            ),
            (
                "until",
                Kind::Until {
                    interval,
                    left: p,
                    right: q,
                },
            ),
            (
                "release",
                Kind::Release {
                    interval,
                    left: p,
                    right: q,
                },
            ),
            (
                "since",
                Kind::Since {
                    interval,
                    left: p,
                    right: q,
                },
            ),
            (
                "triggered",
                Kind::Triggered {
                    interval,
                    left: p,
                    right: q,
                },
            ),
        ] {
            cases.push(graph(format!("{name}-{suffix}"), base(), kind));
        }
    }
    cases.push(graph(
        "previous",
        base(),
        Kind::StrongPrevious { operand: p },
    ));
    let unbounded = TemporalInterval::Unbounded(UnboundedInterval::new(0));
    for (name, inner, outer) in [
        (
            "future-historically",
            Kind::Historically {
                interval: unbounded,
                operand: p,
            },
            Kind::Future {
                interval: unbounded,
                operand: NodeId(2),
            },
        ),
        (
            "historically-future",
            Kind::Future {
                interval: unbounded,
                operand: p,
            },
            Kind::Historically {
                interval: unbounded,
                operand: NodeId(2),
            },
        ),
        (
            "globally-once",
            Kind::Once {
                interval: unbounded,
                operand: p,
            },
            Kind::Globally {
                interval: unbounded,
                operand: NodeId(2),
            },
        ),
        (
            "once-globally",
            Kind::Globally {
                interval: unbounded,
                operand: p,
            },
            Kind::Once {
                interval: unbounded,
                operand: NodeId(2),
            },
        ),
    ] {
        let mut nodes = base();
        nodes.push(InfiniteNode::new(inner));
        cases.push(graph(name, nodes, outer));
    }
    cases
}

fn word(index: usize, cells: Vec<[PartialValue; 2]>, prefix_len: usize) -> V11Word {
    let mut missing = false;
    let mut conflicting = false;
    for value in cells.iter().flat_map(|cell| cell.iter()) {
        missing |= *value == PartialValue::Missing;
        conflicting |= *value == PartialValue::Conflicting;
    }
    let evidence_class = match (missing, conflicting) {
        (false, false) => EvidenceClass::Complete,
        (true, false) => EvidenceClass::Missing,
        (false, true) => EvidenceClass::Conflicting,
        (true, true) => EvidenceClass::Mixed,
    };
    let propositions = [PropositionId(0), PropositionId(1)];
    let observation = |position: usize, values: [PartialValue; 2]| TraceObservation {
        position: u32::try_from(position).expect("finite V11 position"),
        valuation: PartialValuation::new(
            "v11-map".to_owned(),
            &propositions,
            propositions
                .iter()
                .zip(values)
                .map(|(proposition, value)| ValuationEntry {
                    proposition: *proposition,
                    value,
                })
                .collect(),
        )
        .expect("fixed V11 valuation is owner valid"),
    };
    let trace = LassoTraceDocument::new(
        SemanticProfile::InfiniteTraceV1,
        InfiniteClock::EventPosition,
        "v11-map".to_owned(),
        propositions.to_vec(),
        cells[..prefix_len]
            .iter()
            .copied()
            .enumerate()
            .map(|(position, values)| observation(position, values))
            .collect(),
        cells[prefix_len..]
            .iter()
            .copied()
            .enumerate()
            .map(|(offset, values)| observation(prefix_len + offset, values))
            .collect(),
    )
    .expect("fixed V11 lasso is owner valid");
    V11Word {
        index,
        trace,
        evidence_class,
        prefix_len,
        cells,
    }
}

fn cells(
    length: usize,
    mut bits: usize,
    overrides: &[(usize, PartialValue)],
) -> Vec<[PartialValue; 2]> {
    let mut result = vec![[PartialValue::False; 2]; length];
    for slot in 0..2 * length {
        let assigned = overrides
            .iter()
            .find_map(|(index, value)| (*index == slot).then_some(*value));
        result[slot / 2][slot % 2] = assigned.unwrap_or_else(|| {
            let value = if bits & 1 == 0 {
                PartialValue::False
            } else {
                PartialValue::True
            };
            bits >>= 1;
            value
        });
    }
    result
}

fn words() -> Vec<V11Word> {
    let mut cases = Vec::with_capacity(WORD_COUNT);
    for length in 1..=3 {
        for prefix_len in 0..length {
            for bits in 0..(1 << (2 * length)) {
                let index = cases.len();
                cases.push(word(index, cells(length, bits, &[]), prefix_len));
            }
        }
    }
    for length in 1..=2 {
        for prefix_len in 0..length {
            for slot in 0..2 * length {
                for state in [PartialValue::Missing, PartialValue::Conflicting] {
                    for bits in 0..(1 << (2 * length - 1)) {
                        let index = cases.len();
                        cases.push(word(
                            index,
                            cells(length, bits, &[(slot, state)]),
                            prefix_len,
                        ));
                    }
                }
            }
        }
    }
    for prefix_len in 0..2 {
        for bits in 0..4 {
            let index = cases.len();
            cases.push(word(
                index,
                cells(
                    2,
                    bits,
                    &[(0, PartialValue::Missing), (3, PartialValue::Conflicting)],
                ),
                prefix_len,
            ));
        }
    }
    cases
}
