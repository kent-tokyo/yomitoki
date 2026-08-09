# RENSEI architecture (v0.1)

This document defines the crate boundary, public API, report schema, component
interface, scoring direction, confidence/abstention contract, versioning
scheme, and non-goals for RENSEI v0.1, per `AGENTS.md` §32 Task 2. It reflects
what is actually implemented today, not the full spec — see "Non-goals /
deferred" at the end for what's intentionally missing.

## Crate boundary

Single crate, `rensei`, no workspace split. `AGENTS.md` §10 explicitly warns
against over-splitting the workspace in v0.1; there is no large embedded
model yet that would justify separate `rensei-core`/`rensei-models`/
`rensei-cli` crates. That split is revisited when fragment-rarity model files
(Phase 2) exist.

RENSEI depends on `chematic` (registry dependency, not a path dependency) for
all molecule representation, SMILES parsing, ring perception, and
stereochemistry. RENSEI does not reimplement any of that. See "chematic API
surface used" below for exactly what's called.

RENSEI does not depend on RENKIN, and RENKIN must never depend on RENSEI.
RENSEI never runs retrosynthesis search or template application.

## Public API

```rust
pub fn analyze(
    molecule: &chematic::core::Molecule,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, RenseiError>;

pub fn analyze_smiles(
    smiles: &str,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, RenseiError>;
```

`analyze_smiles` is `chematic::smiles::parse` followed by `analyze`. Parsing
is the only fallible step in the whole pipeline — a molecule that parses
successfully always returns `Ok(report)`, never `Err`, no matter how
difficult or out-of-domain it is (`AGENTS.md` §17: a hard-to-synthesize
molecule is not an error).

## chematic API surface used

Confirmed against `chematic 0.11.0` (published on crates.io) by reading
source directly, not guessed:

| Need | chematic API |
|---|---|
| SMILES parsing | `chematic::smiles::parse(&str) -> Result<Molecule, SmilesError>` |
| Molecule struct | `chematic::core::Molecule` (re-exported from `chematic_core`, gated on the `smiles` feature, not its own) |
| Ring perception (SSSR) | `chematic::perception::find_sssr(&Molecule) -> RingSet` |
| Ring system classification | `chematic::perception::find_ring_families(&Molecule, &RingSet) -> Vec<RingFamily>`, `RingFamily.kind: RingSystemKind::{Simple, Fused, Spiro, Bridged}` |
| Valence validation | `chematic::core::validate_valence(&Molecule) -> Vec<ValenceError>` |
| Disconnected fragments | `Molecule::is_connected() -> bool` |
| Stereo completeness | `chematic::perception::stereo_validation::stereo_completeness(&Molecule) -> StereoCompleteness` |

Dependency declaration: `chematic = { version = "0.11", features = ["smiles",
"perception", "chem"] }`. The `chematic` facade crate has `default = []` —
without explicit features it exposes nothing.

Known gaps in chematic's public API (relevant to RENSEI, not filed upstream
yet): no macrocycle predicate in `chematic-perception` (only
`chematic-3d::detect_macrocycle_status`, gated behind the unrelated `threed`
feature); no single unified `sanitize()`/`validate()` entry point (valence,
stereo, and connectivity checks are independent calls); no single top-level
error type spanning all of chematic's functional areas.

## Report schema

```text
SynthesizabilityReport
├── overall: OverallAssessment { synthesizability, difficulty, confidence, verdict }
├── components: ComponentScores        // six Option<ComponentScore> fields
├── findings: Vec<Finding>
├── dominant_penalties: Vec<Contribution>
├── dominant_supports: Vec<Contribution>
├── suggestions: Vec<SimplificationSuggestion>   // always empty in v0.1
├── applicability: ApplicabilityReport
└── provenance: Provenance
```

`ComponentScores` has all six fields from `AGENTS.md` §7
(`size_topology`, `ring_topology`, `stereochemical_burden`, `fragment_rarity`,
`functional_group_liability`, `input_quality`), each typed
`Option<ComponentScore>`. Only `ring_topology` and `input_quality` are `Some`
in v0.1; the rest are `None`. This is a deliberate choice over populating
them with dummy zero scores — `None` says "not evaluated," a zero score would
falsely say "evaluated, found no burden." Going from `Option` to always-`Some`
later is additive; the reverse would be a breaking schema change, so starting
with `Option` is also the safer long-term default.

`Verdict` (§7) defines all six variants (`LikelyAccessible`,
`ModeratelyAccessible`, `Challenging`, `HighlyChallenging`, `Indeterminate`,
`OutOfDomain`) for schema stability, marked `#[non_exhaustive]`. All six are
reachable today: the four accessibility levels come from `ring_topology`'s
normalized burden, `OutOfDomain` from applicability's hard-fail triggers, and
`Indeterminate` when confidence falls below a named threshold without an
outright hard fail.

`FindingCode` (§8.1) is `#[non_exhaustive]` and currently only defines the
codes the two implemented components actually emit: `RingBridgedComplexity`,
`RingSpiro`, `RingFusedDense`, `RingMacrocycle`, `InputUnsupportedElement`,
`InputDisconnected`, `InputUnusualValence`, `InputTooLarge`. Codes for
not-yet-implemented components (e.g. `STEREO_DENSITY_HIGH`, `FRAGMENT_RARE`)
are added when those components are.

Every finding's `explanation: String` is generated from its structured code +
parameters (`explain.rs`), never authored by hand per instance — this keeps
structured data as the source of truth and leaves room for future
localization (§8.3).

## Component interface

Each component module (`components/*.rs`) exposes a `pub(crate) fn compute`
taking `&Molecule` (and `&AnalysisConfig` where config affects the
component, e.g. `max_heavy_atoms`) and returning a `ComponentScore` plus any
component-specific report data. Components do not depend on each other's
output — aggregation happens once, centrally, in `analyze.rs`.

## Scoring direction

* `synthesizability`: 1.0 = easy to make.
* `difficulty`: 1.0 = hard to make.

In v0.1, `difficulty` is computed directly from `ring_topology`'s normalized
burden (the only difficulty-contributing component implemented so far), and
`synthesizability = 1.0 - difficulty`. This complementary relationship is a
v0.1 implementation detail, not a permanent API guarantee — `AGENTS.md` §6
explicitly reserves the right to decouple them once calibration is
introduced. When a second difficulty-contributing component (e.g.
`size_topology`) is added, this becomes a weighted combination with named
weights in `rules.rs`, per §20.

## Confidence contract

`confidence` comes entirely from the `input_quality`/applicability
component: a product of named per-check penalty factors (element coverage,
valence validity, connectivity, stereo completeness — see `rules.rs`), not a
hand-tuned per-molecule number. `AGENTS.md` §5.6 requires confidence and
difficulty to never be conflated — a structurally complex molecule is not
automatically low-confidence; only actual input-quality/applicability
problems lower confidence.

`ring_topology`'s own `ComponentScore.confidence` is fixed at `1.0`: ring
classification via `find_ring_families` is fully deterministic for any
molecule that parsed and passed valence validation, so there's no additional
uncertainty to express there yet. This will stop being a constant once a
component with genuinely variable rule coverage (e.g. fragment rarity, which
depends on corpus coverage) is added.

## Abstention contract

`Verdict::OutOfDomain` fires when the applicability component's hard trigger
fires: disconnected fragments, too high a fraction of unsupported elements,
or heavy-atom count above `AnalysisConfig::max_heavy_atoms`. `analyze` still
returns `Ok(report)` in this case with whatever partial diagnostics were
computable (§21: "abstain時も可能な範囲のpartial diagnosticsを返す") — abstention is
never an `Err`.

`Verdict::Indeterminate` fires when confidence is below a threshold that
depends on `AnalysisConfig::strictness`
(`rules::indeterminate_confidence_threshold`: 0.3 lenient / 0.45 standard /
0.6 strict) without an outright applicability hard fail. The standard
threshold is deliberately set above the confidence floor applicability's two
soft penalties can reach together (0.5 × 0.85 = 0.425) — a threshold at or
below that floor would make `Indeterminate` unreachable at standard
strictness. See `analyze::tests` for the regression tests covering this.

## Versioning

`Provenance` fields and their sources:

| Field | Source |
|---|---|
| `schema_version` | literal constant in `report.rs` |
| `rensei_version` | `env!("CARGO_PKG_VERSION")` |
| `chematic_version` | chematic's declared version requirement |
| `ruleset_version` | `rules::RULESET_VERSION` |
| `config_hash` | SHA-256 (via the `sha2` crate) of the config's canonical JSON serialization |

`config_hash` deliberately does not use `std::hash::DefaultHasher` — that
hasher is randomized per-process on recent Rust versions and would silently
break both determinism (§4.5) and cross-run provenance comparability (§4.6).

## Non-goals / deferred

Not implemented in v0.1 so far (tracked, not stubbed with fake data):

* CLI (§15).
* `size_topology`, `stereochemical_burden`, `fragment_rarity`,
  `functional_group_liability` components.
* Simplification suggestions (§9) — `suggestions` is always an empty `Vec`.
* Fragment corpus, model files, calibration, ML (§19, §28).
* `ApplicabilityReport.domain_distance` — needs a calibration corpus that
  doesn't exist yet; always `None`.
* Python/WASM bindings (§26 Phase 6).

Permanent non-goals (per `AGENTS.md` §2, §28): retrosynthesis planning,
reaction template application, precursor generation, route ranking, yield
prediction, toxicity/hazard (SDS) classification, cost prediction, full
periodic-table or organometallic/polymer support.
