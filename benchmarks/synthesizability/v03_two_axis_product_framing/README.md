# v0.3 Two-Axis Product Framing (round 22 part 23)

Evidence-synthesis-only. No new molecule population scored, no new
dataset opened, no weight/threshold/formula/schema touched. **Zero
production diff** (`git diff main -- src/ tests/ Cargo.toml` returns
nothing). PaRoutes stays permanently spent, per
`v03_reaction_evidence_audit`'s own closing verdict — nothing here
re-opens or re-tests against it. This round evaluates a **product
framing question** — should YOMITOKI conceptually separate intrinsic
structural burden from route/context-dependent difficulty — against
evidence already on record, and picks exactly one of four options.

## Scope note on citations

Half this round's evidence lives on
`experiment/v03-reaction-evidence-information-audit` (commit `1001c44`),
which per this project's own standing decision **stays unmerged
permanently** (REJECT verdict, experimental `[patch.crates-io]`
scaffolding, not production-adopted). Every citation to that work below
names branch + commit explicitly so a reader on `main` can still resolve
it; it will not appear by just browsing `main`'s tree.

## Axis definitions (as scoped for this round)

**Axis A — intrinsic**: explainable from the target molecule alone.
Molecular size/composition, ring topology, stereochemical burden,
structural dimensionality, functional-group liability,
applicability/uncertainty. This is what YOMITOKI's four production
components (`ring_topology`, `size_topology`, `stereochemical_burden`,
`functional_group_liability`) already compute.

**Axis B — route/context**: not determined by the target molecule
alone. Commercial building-block availability, reaction precedent,
reaction-class availability, protecting-group strategy, convergence,
literature/synthetic history, stock definition, planner/search
behavior. Whether YOMITOKI itself should ever compute this axis is
exactly one of this round's open questions (§ Options).

## Terminology note: this round's letters are not the semantic ceiling audit's letters

`v03_semantic_ceiling_audit` (§9) already ran an A/B/C/D decision and
picked **its own "C" — "two-axis conceptual model, qualified."** That
round's options are **not** this round's options:

| | Semantic ceiling audit (round 22 part 21) | This round |
|---|---|---|
| A | Representation redesign, keep route-length as the goal | Intrinsic-only scope narrowing |
| B | Narrow to intrinsic-only, drop route-depth as a goal entirely | Two-axis *report* (intrinsic + explicit context-dependence disclosure) |
| C | Two-axis conceptual separation (representation improvement on the intrinsic side + honest disclosure on the route side) | Ecosystem split (Axis B's home is RENKIN, not YOMITOKI) |
| D | Insufficient evidence to decide | Status quo (keep current single-axis framing) |

Prior-round-C and this-round-B are the closest match (both: keep
computing intrinsic only, but say so explicitly and stop implying
route-depth prediction). Prior-round-B and this-round-A are the closest
match (both: narrow scope, drop the broader claim). **This round's C
has no direct precedent in the prior round** — it's a new, sharper
question (does Axis B belong to YOMITOKI-as-a-second-component, or to
RENKIN-as-a-separate-tool), not a renamed repeat of an earlier decision.
Anywhere below that says "consistent with the prior round," it means
prior-round-C ≈ this-round-B specifically, not a blanket endorsement of
this round's lettering.

## Containment: A, B, and C are not three independent alternatives

Reading the option texts precisely: **B = A's scope narrowing + explicit
two-axis reporting. C = B + a placement claim for Axis B (RENKIN, not
YOMITOKI).** A ⊆ B ⊆ C. "Choose exactly one" is therefore really "choose
how far to commit," not a choice among orthogonal designs — stated
plainly here rather than presenting three unrelated candidates and
picking one by a coin's width.

## Evidence matrix

Every row below is an already-committed, already-frozen result. Nothing
in this table was computed this round.

| Evidence source | What it shows | Axis A implication | Axis B implication | Bears on |
|---|---|---|---|---|
| **MPScore** (chemist-intuition hard/easy labels; `DEVELOPMENT_SET.md` Part 4–6) | Real ring/stereo signal exists and correlates with expert structural-difficulty intuition, but is imperfectly calibrated (`ring_topology` backwards-in-complex-population; stereo-aggregation ceiling in the simple/small stratum) — diagnosis-only, never tuned against directly | Confirms real, non-trivial intrinsic signal | Silent — MPScore labels structural-difficulty intuition, not route outcomes | Q2 |
| **TS1** (GDB-17 cage/ring vs. ZINC15 building blocks; `docs/benchmark.md`) | ROC-AUC 0.952, balanced-acc/MCC exceed SAscore's, roughly match BR-SAScore's | Strong — exactly the structural-contrast regime Axis A is built for | Silent — TS1 is a structural-source contrast, not a route-length or route-existence label | Q2 |
| **TS2** (ChEMBL drug-likes, ≤10-step retrosynthesis-route-found label; `docs/benchmark.md`) | ROC-AUC 0.476, MCC 0.000 — chance level. Per-class `overall.difficulty` distributions nearly identical (0.444 vs 0.433) | Confirms the population is structurally homogeneous by Axis A's own measures | **Direct evidence Axis A alone cannot separate "has a found route" from "doesn't," among structurally similar molecules** | Q1, Q4 |
| **TS3** (`docs/benchmark.md`) | ROC-AUC 0.673 vs. 0.839/0.905 for competitors — weaker but not at chance; root cause out of scope | Partial/weak intrinsic signal, unexplained gap vs. competitors | Not isolated this round | — |
| **Selective prediction / `overall.confidence`** (`docs/benchmark.md`) | AURC shows confidence **actively inverted on TS1**; root cause: confidence is a proxy for dataset provenance (stereo-tag completeness), not for correctness | — | **Rules out leaning Option B's "context-dependence/uncertainty" half on the existing `confidence` field** — it doesn't currently function as a context-dependence signal | Q4 (constrains how B could be implemented) |
| **PaRoutes final holdout** (pre-registered NO-GO; `final_holdout/RESULTS.md`) | Primary endpoint ρ=0.1491 vs. 0.20 floor — NO-GO. v0.2 significantly *worse* than v0.1.1. Collapses to ρ=−0.001 in the high-ring half | Weak but present in the low-complexity half (ρ=0.161) | **Real route-length prediction essentially absent in exactly the population (high-ring) where a route-dependent axis would matter most** | Q1, Q4 |
| **Semantic ceiling audit** (`v03_semantic_ceiling_audit/README.md`) | Both a real representation gap (Morgan/raw descriptors beat YOMITOKI on every slice, ceiling only 0.23 full-set / 0.13 high-ring) AND a real low absolute ceiling (never reaches "moderate," 0.35) for predicting real route depth from any target-only representation. **Stock-similarity to purchasable precursors: ρ=−0.265 in high-ring — larger than any target-only representation achieves there, in either direction** | Real, still-open headroom to improve Axis A's own representation | **Direct, quantified evidence that a meaningful share of real route-length variance lives outside any possible single-molecule representation — a category boundary, not an engineering gap** | Q2, Q3 |
| **Morgan substructure follow-up** (`deb4f7a`) | High-ring gap signal Morgan captures and YOMITOKI misses looks like functional-group/reactive-handle presence (carbonyl, amine, ether, aryl-halide, biaryl context), not finer ring topology | Sharpens *where* Axis A representation could improve | — | Q2 (informs future Axis A work, not this round's decision) |
| **Reaction-evidence audit — REJECT** (`experiment/v03-reaction-evidence-information-audit@1001c44`, `benchmarks/synthesizability/v03_reaction_evidence_audit/README.md`) | Testing the Morgan-substructure lead directly (`retro_disconnect`-derived reaction-context evidence added to F0): small real full-set effect (paired bootstrap CI excludes zero), but **high-ring — the stratum this round was opened to help — both CIs include zero.** PaRoutes now permanently spent | Confirms Axis A improvement in the high-ring regime is not simply "add reaction-template evidence" — that specific lead is now closed | Reinforces that closing the high-ring gap needs route/context information (stock, precedent), not more target-only features | Q1, Q3, Q4 |

**What the matrix rules out**: Option D (status quo) — TS2 at chance,
PaRoutes NO-GO with high-ring collapse, and the reaction-evidence REJECT
all triangulate on the same conclusion: a single undifferentiated score
does not, and per the now-spent PaRoutes evidence and the closed
reaction-evidence lead, is not going to via further target-only tuning,
reliably track real synthetic difficulty in general — continuing to
iterate a single score against this evidence base has hit a wall two
consecutive rounds already found independently.

**What the matrix rules out for YOMITOKI computing Axis B itself**: the
stock-similarity finding (ρ=−0.265, high-ring) is not "YOMITOKI hasn't
gotten to this yet" — it's evidence that the deciding factor requires
information (a purchasable-inventory index) that structurally cannot
come from a single target molecule, by construction. Any future
YOMITOKI component trying to compute Axis B would need exactly the kind
of stock/precedent/planner data that defines RENKIN's job.

**What the matrix is silent on**: RENKIN doesn't exist yet as running
code. Nothing above measures RENKIN specifically — Option C's "→
RENKIN" half is a placement decision, not a measurement. It rests on a
boundary this project has already declared standing in two prior
rounds (semantic ceiling audit §10, reaction-evidence audit §8, both:
"`chematic → yomitoki → renkin` stays intact") and in the currently
shipped `README.md` ("yomitoki never runs route search — that boundary
is permanent, not a v0.1 scoping choice"; "What it does not do" lists
retrosynthesis planning/route ranking as RENKIN's job). Option C is
therefore **ratifying and sharpening an already-standing, already-
enforced product boundary**, not proposing a new architectural bet.

## Six key questions

**Q1 — Does YOMITOKI claiming to predict "synthetic difficulty" in
general overclaim?**
Not in the README's prose taken as a whole: it already leads with
"route-free" (`README.md:10,12`), states the RENKIN boundary as
permanent (`README.md:76-77`), and lists retrosynthesis
planning/route ranking under "What it does not do." But there is a
**terminology risk below the prose layer**: the public struct fields
are named `overall.difficulty` / `overall.synthesizability`
(`src/report.rs:301-306`) with no "intrinsic" qualifier, and the CLI
output literally prints "Synthesizability: 0.66" with no scope caveat
inline. A user who reads the API surface without the full README could
reasonably read "difficulty" as "how hard this really is to make,"
which TS2 (chance-level on a route-existence label) and PaRoutes
(ρ=0.149, collapsing to ~0 in high-ring) directly show it doesn't
validate. **Verdict: not an overclaim in the documented whole, but a
real gap between what the prose discloses and what the field names by
themselves imply** — exactly the gap Option B/C's explicit two-axis
reporting would close.

**Q2 — Does an "intrinsic structural synthesizability" framing align
with existing evidence?**
Yes, well. TS1's 0.952 ROC-AUC on a structural-contrast population and
MPScore's real (if imperfectly calibrated) ring/stereo signal both
validate genuine structural signal when the question is about intrinsic
burden rather than real route length. This is exactly the claim the
evidence supports; it is a narrower, more defensible claim than
"synthetic difficulty" unqualified.

**Q3 — Would YOMITOKI itself owning a route/context axis duplicate
RENKIN's responsibility?**
Yes, and the evidence says more than "duplicate" — it says
**structurally cannot, not just shouldn't**. The single strongest
predictor found anywhere in the high-ring stratum, across every round
in this line of work, is stock-similarity to purchasable precursors
(ρ=−0.265) — information that by definition requires a
purchasable-inventory index outside the target molecule. That is
RENKIN's domain (stock/planner data), not a richer molecule
representation YOMITOKI could compute its way into.

**Q4 — Is a two-axis report more explanatory to users than a single
score?**
Given the pattern (real intrinsic signal + weak-to-chance real-route
correlation, worst exactly where users most want an answer — complex,
high-ring molecules), yes: an undifferentiated score risks being read
as "predicts real step count," which TS2 and PaRoutes directly
contradict. One caveat retrieved this round: **the existing
`overall.confidence` field cannot be repurposed as the
"context-dependence/uncertainty" half of a two-axis report as-is** — it
is proven actively inverted on TS1 and is a proxy for dataset
provenance (stereo-tag completeness), not for how much route-context
would matter (`docs/benchmark.md`, "Selective prediction"). A two-axis
report needs either a plain textual/documentation disclosure (cheap,
no schema change) or a genuinely new signal — not a repurposing of
`confidence`.

**Q5 — Does the current `overall.difficulty` name itself eventually
need to change?**
The project has already anticipated this. `src/report.rs:298-299`:
*"`synthesizability`/`difficulty` are complementary in v0.1 as an
implementation choice, not a permanent API guarantee"* — directly
citing `AGENTS.md` §6, which independently declines to permanently
guarantee `difficulty = 1 − synthesizability` for the same reason (a
future calibration might not preserve it cleanly). A future rename
toward something like `intrinsic_difficulty` (either replacing or
sitting alongside the current fields) is a small, already-foreseen
move — not a new precedent this round would be inventing. Not needed
now (schema changes are explicitly out of scope this round) but the
architectural room for it already exists.

**Q6 — Stable-API-compatibility implications?**
yomitoki is pre-1.0 (`0.2.0-alpha.1`) with no documented "no breaking
changes" commitment found anywhere in `README.md`/`docs/architecture.md`
— under ordinary 0.x semver convention every 0.x bump can already be
breaking. The mechanism a genuine field rename would actually touch is
`Provenance.schema_version` (currently `0.5.0`,
`docs/architecture.md:641`), not `RULESET_VERSION` (a rename carries no
scoring-behavior change). **Cost today: low** — alpha status, no
external stability promise yet, and the change is already anticipated
in a doc comment. **Cost grows once a stable 1.0 is declared** — after
that point a rename needs either an additive field kept alongside the
old one through a deprecation window, or a major bump. Practical
implication: if Option C is adopted, doing the naming/reporting work
**before** any 1.0 stability declaration is cheaper than after — but
that is a scheduling note, not an action for this round (no schema
change is authorized here).

## Option analysis

**A — Intrinsic-only YOMITOKI.** Scope narrowed explicitly to
"route-free intrinsic structural synthesizability diagnostics";
route-dependent difficulty deliberately not predicted or claimed.
*Strength*: cheapest, most honest floor — mostly already true in
prose, would mainly need field-name/doc tightening (Q1, Q5). *Weakness*:
says nothing about where a user should go for route-dependent
questions, and drops the still-real, still-open Axis-A representation
lead (Morgan-substructure functional-group signal) from the framing
entirely, even though nothing here says stop pursuing it.

**B — Two-axis YOMITOKI report.** Still computes only Axis A, but the
report conceptually separates intrinsic difficulty from
context-dependence/uncertainty; still no route search. *Strength*: most
directly answers Q1's terminology gap and Q4's explanatory-value
question — a user gets both "here's the intrinsic read" and "here's
what this doesn't cover and why," without YOMITOKI taking on new scope.
*Weakness*: as retrieved this round (Q4), the natural existing vehicle
for the "context-dependence" half (`overall.confidence`) is proven
unusable for that purpose — this option requires either new
documentation-only disclosure (cheap) or a genuinely new field (a
schema change, explicitly out of scope this round), so its cheapest
honest version is documentation, not a new report shape, until a future
round does that scoping work.

**C — Ecosystem split.** YOMITOKI intrinsic-only; route-dependent
evidence lives in RENKIN or a future joint evaluation layer
(`chematic → yomitoki (intrinsic structural diagnostics) → renkin
(route-dependent evidence/planning)`). *Strength*: matches Q3's
strongest finding (Axis B needs stock/precedent data YOMITOKI
structurally cannot see) and ratifies a boundary two prior rounds
already declared standing and the current README already documents in
practice — the architecture is not new, only the explicit naming of
*why* is. *Weakness*: RENKIN doesn't exist yet as running code, so the
"→ renkin" half is a placement/roadmap claim, not something this
round's evidence can measure directly (disclosed above, under the
evidence matrix) — its correctness rests on the boundary decision being
right, not on new data confirming RENKIN specifically.

**D — Status quo.** Ruled out by the evidence matrix: TS2 at chance,
PaRoutes NO-GO with high-ring collapse to ρ≈0, and the reaction-evidence
REJECT (the specific, honestly-tested attempt to close that exact gap)
all triangulate on the same wall. Continuing single-axis framing
without disclosure carries forward the Q1 terminology risk with no
mitigation.

## Recommendation: C

Since A ⊆ B ⊆ C, recommending C means recommending A and B's content
too, plus the explicit RENKIN placement. Rationale, in order of weight:

1. **Q3's finding is the strongest, most direct evidence in this
   round's entire base**: stock-similarity beating every target-only
   representation in the hardest stratum is not an argument that
   YOMITOKI *should* stay narrow — it is evidence that it
   *structurally cannot* do otherwise without becoming a different kind
   of tool (one that needs stock/planner data). That is a category
   boundary, and C is the only option that names it explicitly.
2. **C is not a new bet.** `chematic → yomitoki → renkin` is already a
   standing decision (semantic ceiling audit §10, reaction-evidence
   audit §8) and already partly documented in the shipped `README.md`.
   Adopting C mostly means finishing work already implicitly underway
   — tightening terminology (Q1, Q5) and making the axis split and its
   evidentiary basis explicit — not reorganizing the project.
3. **It matches the evidence-driven verdict of the prior round most
   directly comparable** — `v03_semantic_ceiling_audit`'s own "C,
   qualified" (which, per the terminology-collision note above, maps
   onto this round's B, the intrinsic-content half of this
   recommendation) reached the same substantive conclusion via
   independent evidence (representation gap + low ceiling + stock
   sensitivity) before this round's reaction-evidence work closed off
   the one remaining lever (reaction-template evidence) that might have
   changed the picture.
4. **D is excluded by three independent evidence sources** (TS2, high-
   ring PaRoutes, reaction-evidence REJECT), not by preference.

This matches the user's own stated prior going into this round (C,
explicitly framed as a hypothesis to be falsified against evidence, not
a conclusion) — recorded here as agreement reached independently via
the evidence matrix above, not as circular confirmation of the prior
itself.

## Proposed v0.3 product contract (one paragraph)

YOMITOKI computes and reports **intrinsic structural synthesizability**
only — burden explainable from the target molecule's own structure
(size, ring topology, stereochemistry, functional-group liability,
input-quality/applicability) — and does not predict, and its report
should say plainly does not predict, real-world route-dependent
synthesis difficulty (precursor availability, protecting-group
strategy, convergence, reaction precedent, or historical route choice);
that axis is out of YOMITOKI's structural scope by design, not by
current limitation, and belongs to RENKIN (or a future joint evaluation
layer sitting above both tools) once it exists. The chematic → yomitoki
→ renkin boundary stays permanent.

## What this round explicitly did not do

No component added. No weight changed. No threshold changed. No
reintroduction of `retro_disconnect`. No `RULESET_VERSION` bump. No API
schema change. No release. `overall.difficulty`/`overall.synthesizability`
field names are unchanged; `Provenance.schema_version` is unchanged
(`0.5.0`). Zero diff to `src/`, `tests/`, `Cargo.toml`.

## Chematic 0.14.1 release — status (Part A of this round's brief)

Already fully shipped, directly by the user, before this round's yomitoki-side
work began — no release action was taken in this session. Verified this
round, read-only:

- Release commit `aead3f7` ("release: v0.14.1"), merged via PR #311
  (`864e256`) to chematic's `main`.
- Tag `v0.14.1` present on `origin` and points at the release merge
  commit.
- GitHub Release published (`2026-08-12T10:38:37Z`), not a draft, not a
  prerelease.
- Release workflow run `31588244018` completed successfully (1m9s).
- crates.io: `chematic` and all workspace-publishable sub-crates
  checked (`chematic-core`, `chematic-rxn`, `chematic-mol`,
  `chematic-chem`, `chematic-wasm`, `chematic-smarts`) live at
  `0.14.1`. `chematic-py` is deliberately excluded from crates.io
  (`publish = false` in its own `Cargo.toml`) and ships via PyPI
  instead — confirmed `chematic` `0.14.1` live on PyPI.
- docs.rs: built successfully for `0.14.1` (`doc_status: true`).
- `all_default_templates_parse`, `test_suzuki_biaryl_matches_real_biaryl_bond`,
  and `test_suzuki_biaryl_does_not_match_intra_ring_bonds` all pass on
  current chematic `main`.
- `CHANGELOG.md`'s `[0.14.1]` entry discloses the 10-template
  constraint-broadening caveat explicitly ("10 templates, rewritten to
  the supported subset with disclosed precision tradeoffs") and does
  not claim full SMARTS semantics anywhere in that section — both
  brief requirements satisfied.

**Two facts worth flagging plainly, not silently absorbed into "done":**

1. **Scope mismatch.** This round's brief described 0.14.1 as a patch
   release scoped to the #296 template-parse fix. What actually shipped
   also bundles an unrelated platinum-coordination-chemistry
   compatibility set (dative-bond valence fix, full periodic-table mass
   data, Extended XYZ read/write) — the #296 fix "happened to land in
   the same release window" per the CHANGELOG's own words for why it's
   filed under `[0.14.1]` rather than `[0.14.0]`/`[Unreleased]`.
2. **Self-disclosed semver tension.** The `[0.14.1]` CHANGELOG entry
   states its own extxyz-related `XyzFrame`/`XyzError`/`write_extxyz`
   Rust API signature changes are *"a real break to the v0.14.0 Rust
   API surface already published to crates.io, not merely a break to
   something unreleased"* — shipped under a patch (0.14.0→0.14.1)
   version bump. This is a fact about what already shipped, not
   something this round has authorization to act on (no new chematic
   release was requested or performed here) — surfaced for the record,
   not for action.

Main is now ahead of the `v0.14.1` tag (PRs #307–#313: platinum
benchmark corpus, dative-bond/periodic-table fixes, extxyz, symmetric
RMSD oracle, topological equivalence classes, torsion-motif diagnostic,
square-planar stereochemistry, an RFC-archive docs move, and a
jsonschema dependency bump) — normal accumulation toward whatever comes
after 0.14.1, not something this round has any brief to act on.

## YOMITOKI reaction-evidence branch (Part B of this round's brief)

Unchanged, as instructed: `experiment/v03-reaction-evidence-information-audit`
(`567872c`) stays unmerged, retained as historical evidence. Not
touched this round beyond being read as an evidence source (§ Evidence
matrix).

## Next step

Exactly one: draft the terminology/documentation changes Option C
implies (README "route-free" → explicit "intrinsic structural
synthesizability" language, an explicit RENKIN-boundary rationale
paragraph citing this round's stock-sensitivity/TS2 evidence, and a
CLI/doc-level disclosure that `overall.difficulty` measures intrinsic
burden only) as its own small, reviewable, docs-only change-set in a
future round — not in this one, and not silently folded into this
README.

## Git

Branch: `experiment/v03-two-axis-product-framing` (from `main` at
`deb4f7a`). This file is the only change. Committed on this branch;
push/merge held per standing discipline.
