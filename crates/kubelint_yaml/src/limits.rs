//! The per-document limits `kubectl` enforces while reading YAML, so that kubelint stops
//! at the same documents and a few hundred bytes cannot exhaust memory or the stack.

/// Kubectl's JSON decoder (`maxNestingDepth` in `encoding/json`) refuses more than this
/// many nested collections, the root included.
pub(crate) const MAX_DEPTH: usize = 10_000;

const ALIASES_MESSAGE: &str = "aliases expand to too many nodes";

/// Counts the nodes `go-yaml` v2's decoder visits in one document and the share that
/// comes from alias expansion (`decodeCount` and `aliasCount` in `decode.go`).
#[derive(Default)]
pub(crate) struct AliasBudget {
    visits: u64,
    aliased: u64,
}

impl AliasBudget {
    pub(crate) fn visits(&self) -> u64 {
        self.visits
    }

    /// Counts one node. Fails once the document is over the budget.
    pub(crate) fn visit(&mut self) -> Result<(), &'static str> {
        self.visits += 1;
        self.check()
    }

    /// Counts the `weight` visits an alias expansion makes, before it is cloned. The
    /// ratio only grows during an expansion, so checking its end is exact.
    pub(crate) fn expand(&mut self, weight: u64) -> Result<(), &'static str> {
        self.visits += weight;
        self.aliased += weight;
        self.check()
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "go-yaml divides the counts as float64 too"
    )]
    fn check(&self) -> Result<(), &'static str> {
        let (aliased, visits) = (self.aliased as f64, self.visits as f64);
        let over =
            self.aliased > 100 && self.visits > 1000 && aliased / visits > allowed_ratio(visits);
        if over { Err(ALIASES_MESSAGE) } else { Ok(()) }
    }
}

/// `allowedAliasRatio` in `decode.go`: 0.99 up to 400,000 visits, 0.10 from 4,000,000
/// and linear between.
fn allowed_ratio(visits: f64) -> f64 {
    const LOW: f64 = 400_000.0;
    const HIGH: f64 = 4_000_000.0;
    if visits <= LOW {
        0.99
    } else if visits >= HIGH {
        0.10
    } else {
        0.99 - 0.89 * ((visits - LOW) / (HIGH - LOW))
    }
}
