# Development set: TS2/TS3 failure-mode diagnosis (round 22, part 2)

This document is the pre-registered spec + first diagnosis for a
development set, separate from TS1/TS2/TS3, built to answer one
question: **why is `overall.difficulty` at chance level on TS2 (ROC-AUC
0.476) and weaker than competitors on TS3 (0.673)?** — without touching
TS1/TS2/TS3 again, per the test-set-integrity rule in
[`../../docs/benchmark.md`](../../docs/benchmark.md).

**This document and the panel it describes carry no easy/hard ground
truth. Nothing here can be used to compute accuracy. Its only purpose is
to localize which of yomitoki's five components responds to which
structural axis** — diagnosis, not tuning. No weight, threshold, or
formula was changed this round. Any future scoring change must be
designed against this (or a similarly-built) development set and
confirmed only afterward against TS1/TS2/TS3, labeled post-hoc.

## Baseline (frozen, for future before/after comparison)

The committed benchmark numbers this document diagnoses are pinned at
commit `2fce5c0`. Do not copy them to a second file — read them from git:

```
git show 2fce5c0:benchmarks/synthesizability/results/benchmark_summary.json
```

Any future round that changes scoring and re-runs TS1/TS2/TS3 must diff
against that exact commit's numbers, not against whatever
`results/benchmark_summary.json` happens to contain at the time (that
file is regenerated in place and is not itself a historical record — git
history is).

## Part 1: TS2/TS3 failure-mode classification

This is retrospective analysis of already-seen, already-committed TS1/2/3
results (legitimate — the test-set-integrity rule forbids using TS1/2/3
to *tune* parameters, not to *understand* why an already-final,
already-published result came out the way it did). No TS1/2/3 molecule
identity below is used to select development-set molecules; only the
aggregate structural pattern is.

At the frozen `overall.difficulty >= 0.5` threshold, TS2's confusion
matrix is exactly symmetric (TP=331, TN=569, FP=331, FN=569) — a
first hint that `overall.difficulty` may simply lack *spread* across
TS2's drug-like population rather than pointing the wrong direction
entirely (its per-class means are 0.444 vs. 0.433, see
`docs/benchmark.md`). Stratifying by RDKit descriptors and yomitoki's own
`findings` codes on TS2 (n) and TS3 (n) sharpens this into two distinct,
opposite-direction failure modes:

| category | TS2 n | TS2 heavy atoms | TS2 rings | TS2 stereo | TS2 aromatic rings | TS3 n | TS3 heavy atoms | TS3 rings | TS3 stereo |
|---|---|---|---|---|---|---|---|---|---|
| TP (correct "hard") | 331 | 19.9 | 2.9 | 2.95 | 0.96 | 748 | 31.9 | 4.3 | 3.82 |
| TN (correct "easy") | 569 | 19.6 | 2.2 | 0.50 | 1.66 | 376 | 21.0 | 2.3 | 0.48 |
| **FP** (said hard, actually easy) | 331 | **30.2** | **4.0** | 0.56 | **3.05** | 371 | **32.5** | **4.3** | 1.03 |
| **FN** (said easy, actually hard) | 569 | **15.8** | **1.6** | **1.74** | 0.54 | 304 | **20.6** | 2.4 | **1.17** |

Finding-code firing rates sharpen it further (TS2 shown; TS3 the same
shape, weaker):

| finding | TP | TN | **FP** | **FN** |
|---|---|---|---|---|
| `SIZE_LARGE_MOLECULAR_WEIGHT` | 10% | — | **20%** | — |
| `FUNCTIONAL_GROUP_DENSE` | 5% | 3% | **15%** | — |
| `STEREO_CENTER_COUNT` | 81% | 26% | 30% | **57%** |
| `RING_BRIDGED_COMPLEXITY` | 36% | — | 1% | 3% |

### FM-1: large, ring-rich, polyaromatic molecules over-penalized

TS2/TS3's false positives are large (heavy atoms +50% over TN), ring-rich
(+80%), and heavily aromatic (+84% aromatic ring count over TN) — but
**not** bridged/spiro/macrocyclic (`RING_BRIDGED_COMPLEXITY` fires at 1%
for FP, vs. 36% for TP). Chemically: fused *flat* aromatic scaffolds
(biaryls, fused heteroarenes) are frequently trivial to assemble from
common building blocks via well-precedented aromatic chemistry
(cross-coupling, SNAr, cyclization) even when they contain many rings —
exactly the population where a retrosynthesis planner finds a short
route despite the raw ring/atom count being high. Hypothesis: yomitoki's
`ring_topology`/`size_topology` respond to raw ring-fusion *count* and
molecular size without distinguishing "flat, well-precedented aromatic
fusion" from genuine 3D ring complexity.

### FM-2: small, stereocenter-bearing molecules under-penalized

TS2/TS3's false negatives are smaller than TN (fewer heavy atoms, fewer
rings) but carry meaningfully more stereocenters (`STEREO_CENTER_COUNT`
fires at 57% for FN vs. 26% for TN). Hypothesis: a molecule's low
ring/size contribution can keep `overall.difficulty` under the 0.5
verdict threshold even when its stereochemical burden is real, because
reaching "challenging" requires ring/size contribution that a small,
few-ring molecule structurally cannot supply — a **threshold/aggregation**
effect, not necessarily evidence that `stereochemical_burden` itself is
underweighted (tested directly in Part 3 below).

## Part 2: development-set sampling specification

Fixed **before** building any data, per the round's instruction not to
select samples that could hide or exaggerate TS2's chance-level result or
the confidence inversion. The selection rule is the generator code
itself (`scripts/ablation_panel.py`), not a prose description of one —
every molecule is a fixed SMILES literal in that file, each with an
asserted RDKit-descriptor target checked at run time (wrong construction
fails loudly, not silently).

**No molecule here carries an easy/hard label.** Instead of trying to
reproduce TS2/TS3's Retro*-based ground truth (not reproducible without
the same retrosynthesis-planner infrastructure BR-SAScore used, and
fabricating labels from recalled literature/drug-approval status would
be unverifiable and was explicitly rejected this round — see the
mid-round design discussion), this panel isolates **structural axes**
and reads yomitoki's own component-level response to controlled
variation along each one. This directly tests the FM-1/FM-2 hypotheses
above without needing external ground truth: if FM-1's hypothesis is
right, `ring_topology`/`size_topology` should climb steeply with fused
flat-aromatic ring count alone; if FM-2's hypothesis is right, `overall.difficulty`
should stay under 0.5 even as stereocenter count climbs, as long as ring
count stays low.

Three axes, twelve molecules total:

- **FM-1 axis** (`FM1_*`, 4 molecules): fused aromatic ring count 1→4
  (benzene → naphthalene → anthracene → tetracene), stereocenters = 0,
  no bridging/spiro throughout. Isolates ring-fusion count.
- **FM-2 axis** (`FM2_*`, 5 molecules): stereocenter count 0→4, ring
  count = 1 throughout, heavy-atom count grows only modestly (10→15, from
  the added halogen substituents themselves, not from added rings).
  Isolates stereocenter count in an otherwise-simple molecule.
- **CTRL axis** (`CTRL_*`, 3 molecules, not a failure-mode target):
  bridgehead atom count 0/2/4 (cyclohexane → norbornane → adamantane) —
  genuine 3D ring complexity, the kind `ring_topology` is actually meant
  to detect. If the score does **not** respond here either, that would
  mean the components are blind, not merely miscalibrated — a
  meaningfully worse finding than FM-1/FM-2 alone would show. Included
  specifically to distinguish those two possibilities.

**Zero overlap with TS1/TS2/TS3, verified not assumed**: every panel
molecule's canonical SMILES is checked against all three benchmark sets'
5398 unique canonical SMILES before the panel is allowed to run
(`ablation_panel.py`'s `verify_no_overlap_with_benchmark_sets` — refuses
to proceed on any hit). Result this round: 0/12 overlap.

Reproduce: `python3 scripts/ablation_panel.py` (needs the release binary
built: `cargo build --release --bin yomitoki` from the repo root). Full
per-component output: `results/ablation_panel.jsonl` (committed — twelve
hand-constructed textbook molecules, no license concerns, unlike
TS1/TS2/TS3).

## Part 3: first diagnosis

`overall.difficulty` and each component's `contribution` (already
weighted, on the same 0-1-ish scale as `overall.difficulty` itself):

| id | difficulty | ring | size | stereo | fg |
|---|---|---|---|---|---|
| FM1_R1 (benzene) | 0.1044 | 0.0952 | 0.0232 | 0 | 0 |
| FM1_R2 (naphthalene) | 0.2743 | 0.2592 | 0.0377 | 0 | 0 |
| FM1_R3 (anthracene) | 0.3316 | 0.2800 | 0.0521 | 0 | 0.0769 |
| FM1_R4 (tetracene) | 0.3486 | 0.2914 | 0.0662 | 0 | 0.0769 |
| FM2_S0 (0 stereo) | 0.1285 | 0.0952 | 0.0834 | 0 | 0 |
| FM2_S1 (1 stereo) | 0.1854 | 0.0952 | 0.0883 | 0.1098 | 0 |
| FM2_S2 (2 stereo) | 0.2186 | 0.0952 | 0.0977 | 0.1071 | 0.0769 |
| FM2_S3 (3 stereo) | 0.3432 | 0.0952 | 0.1188 | 0.2827 | 0.1479 |
| FM2_S4 (4 stereo) | **0.4212** | 0.0952 | 0.1676 | 0.3473 | 0.2134 |
| CTRL_B0 (cyclohexane) | 0.1051 | 0.0952 | 0.0249 | 0 | 0 |
| CTRL_B2 (norbornane) | 0.3411 | 0.3297 | 0.0284 | 0 | 0 |
| CTRL_B4 (adamantane) | 0.3457 | **0.3297** | 0.0400 | 0 | 0 |

**FM-1 hypothesis confirmed directly**: `ring_topology` climbs steeply
with fused aromatic ring count alone (0.095 → 0.259 → 0.280 → 0.291) with
no stereocenters, no bridging, nothing else changing — going from
benzene to naphthalene (a 4-ring jump would still be an everyday,
cheap, two-step-or-fewer building block) very nearly doubles
`overall.difficulty` (0.104 → 0.274) on ring-fusion count alone. This
directly supports FM-1: `ring_topology` does not distinguish "many flat,
well-precedented fused-aromatic rings" from genuine structural
complexity — it responds to fusion *count*, unbounded (still climbing,
not yet plateaued, at 4 rings/tetracene).

**FM-2 hypothesis confirmed, and sharpened**: `stereochemical_burden`
*does* respond roughly proportionally to stereocenter count (0 → 0.110 →
0.107 → 0.283 → 0.347) — it is not ignored. But `overall.difficulty`
for FM2_S4 (four genuine, contiguous stereocenters, otherwise the
simplest possible molecule: one ring, no bridging, no aromaticity)
still lands at **0.4212 — under the 0.5 "challenging" threshold**, with
`ring_topology`'s contribution pinned at 0.0952 throughout the entire
axis (ring count never changes). This is the precise mechanism behind
FM-2: it is not that stereo is underweighted per stereocenter, it is
that **`overall.difficulty`'s aggregate needs simultaneous contribution
from ring/size to cross the "challenging" threshold, and a low-ring-count
molecule structurally cannot supply that no matter how stereochemically
loaded it is.** This matches TS2/TS3's FN profile exactly (small, few
rings, real stereocenters, mislabeled "easy" by yomitoki).

**CTRL axis: components are not blind, but saturate on bridged
complexity specifically**: `ring_topology` responds sharply to the first
bridgehead pair (cyclohexane 0.095 → norbornane 0.330), ruling out "the
components don't see 3D complexity at all." But it is **flat between
norbornane and adamantane** (0.3297 in both cases) despite adamantane
being an unambiguously more complex, more caged system with double the
bridgehead count. `ring_topology` treats "bridged" as close to a binary
signal rather than scaling with degree of bridged complexity — a real,
separate finding from FM-1's unbounded-with-count behavior on flat
fusion, not investigated further this round.

## Part 4 (round 22, part 3): validation against the MPScore expert-chemist dataset

Follow-on round. Full dataset provenance, license, rater-coverage,
consensus-construction, and TS1/2/3-overlap investigation is in
[`../datasets/README.md`](../datasets/README.md)'s "MPScore" section —
summary: `stevenkbennett/synthetic_accessibility_project` (MIT, pinned
tag `1.0.6`), three chemists' independent easy/difficult ratings on
10,966 molecules (after excluding 3 exact TS1/2/3 overlaps), 86% rated
by exactly one chemist, only 41 by all three. This dataset's ground
truth is **human chemist intuition**, methodologically independent from
TS1/TS2/TS3's Retro*-route-existence labels — it was investigated
specifically to test whether TS2's chance-level result and the FM-1/FM-2
failure modes are artifacts of one labeling methodology or a more
general weakness. Frozen v0.1.0 baseline, no weight/threshold/formula
changed. Reproduce: `python3 scripts/download_mpscore.py
datasets/downloaded && python3 scripts/run_yomitoki.py
datasets/downloaded/mpscore/mpscore_dev.smi results/raw/mpscore_yomitoki.jsonl
&& python3 scripts/evaluate_mpscore.py`.

### Headline: TS2's chance-level result reproduces on an independent, methodologically unrelated dataset

| population | n | ROC-AUC (95% CI) | PR-AUC | Balanced Acc. | MCC |
|---|---|---|---|---|---|
| Full (all defined consensus) | 10,543 | 0.513 [0.496, 0.529] | 0.863 | 0.497 | -0.006 |
| n_raters ≥ 2 (real agreement behind the label) | 1,110 | 0.487 [0.444, 0.532] | 0.816 | 0.473 | -0.049 |
| n_raters = 3 (all three chemists) | 41 | 0.438 [0.258, 0.624] | 0.445 | 0.458 | -0.092 |

Every 95% CI contains 0.5 — statistically indistinguishable from random
guessing, not distinguishable-but-inverted (the point estimates run
slightly below 0.5, but the CIs are wide enough, especially for n=41,
that "inverted" would overclaim). This is the same qualitative result
TS2 already showed (ROC-AUC 0.476, also chance-level) on a dataset with
0.03% exact-molecule overlap, a different labeling methodology (human
judgment vs. retrosynthesis-planner route search), a different label
skew (16%/84% easy/hard here vs. TS2's balanced 50/50), and a different
originating research context (porous-organic-cage precursor screening
vs. general ChEMBL-derived molecules). **Two independent lines of
evidence now point the same direction: yomitoki's four structural
components, as currently weighted and aggregated, do not track
synthesis difficulty on drug-like/precursor-like populations — this is
not an artifact of TS2's specific label source.**

PR-AUC (0.863) looks high in isolation but is not evidence of real
skill here — PR-AUC is inflated by MPScore's 84% "hard" base rate
(a classifier that always predicts "hard" scores PR-AUC ≈ 0.84 on this
class balance with zero discriminative power). ROC-AUC and MCC, both
base-rate-insensitive, are the metrics that matter, and both say
"no signal."

### The dominant error is false negatives, not the FM-1/FM-2 split alone

Confusion matrix (full set, n=10,543, threshold 0.5): TN=1,147,
**FP=236**, **FN=7,652**, TP=1,508. 72.6% of the entire dataset is a
false negative — yomitoki calls the molecule accessible, the chemist
called it difficult. This dwarfs the false-positive count (236, 2.2%)
that FM-1 explains. Component means confirm the shape: `ring_topology`'s
mean contribution is actually *slightly higher* for the chemist-easy
class (0.206) than the chemist-hard class (0.184) on this dataset —
backwards, not just weak. `stereochemical_burden` is in the right
direction (0.079 hard vs. 0.030 easy) but small in absolute terms.
**The honest reading: on MPScore, a large majority of what chemists call
"difficult" is invisible to all four of yomitoki's current components,
not merely miscalibrated within them.** Plausible missing factors (not
investigated further this round — would need a citable primitive, not
guessed at): functional-group *compatibility* in a specific reaction
context (this dataset's rating exercise was conducted around
cage-forming imine/aldehyde chemistry), purification difficulty,
known-troublesome reagent classes — categories of "hard" a structural
topology score was never designed to see.

### FM-1 replicates; FM-2/FM-3 are present but not decisive

Descriptor means by confusion-matrix category (full set):

| category | n | heavy atoms | rings | aromatic rings | stereocenters | bridgeheads |
|---|---|---|---|---|---|---|
| TP (correct "hard") | 1,508 | 21.5 | 3.54 | 1.03 | 3.68 | 0.99 |
| TN (correct "easy") | 1,147 | 15.6 | 1.79 | 1.52 | 0.60 | 0.04 |
| **FP** (said hard, actually easy) | 236 | **38.8** | **5.61** | **4.60** | 0.94 | 1.34 |
| **FN** (said easy, actually hard) | 7,652 | 14.6 | 1.60 | 0.87 | 0.90 | 0.09 |

**FM-1 (large/ring-rich/aromatic over-penalization) replicates
directly**: FP molecules are the largest (38.8 heavy atoms, nearly
double TP's 21.5), most ring-rich (5.61 rings), and most aromatic (4.60
aromatic rings) of any category — the same signature found on TS2/TS3.
One nuance not seen on TS2/TS3: here FP's bridgehead mean (1.34) is
*higher*, not lower, than TP's (0.99) — on TS2/TS3, FP was specifically
*non*-bridged. FM-1 is therefore better stated as "large, ring-rich,
highly-fused molecules are over-penalized" rather than narrowly "*flat*
fused-aromatic-only" — the flat-vs-bridged distinction that looked sharp
on TS2/TS3 does not hold as cleanly here. Reported as a partial, not a
full, replication.

**FM-2 representation check**: 41.8% of the usable set (4,404/10,543)
has at least one stereocenter — well-represented, not sparse. FN's mean
stereocenter count (0.90) is higher than TN's (0.60), directionally
consistent with FM-2, but small relative to TP's (3.68), and FN is
numerically dominated by the base-rate mismatch described above, not by
stereocenter-rich molecules specifically. The ablation panel's
mechanistic finding (round 22 part 2: four stereocenters alone,
holding ring count at 1, cannot cross the 0.5 threshold) stands on its
own as a controlled result; its *share* of MPScore's real-world error is
small.

**FM-3 representation check**: 8.9% of the usable set (937/10,543) has
any bridgehead or spiro atom — present, but too sparse for a clean
dose-response curve, and confounded by the FP-vs-TP pattern above (both
elevated-bridging categories, cutting across the hard/easy label rather
than separating by it). Bridging correlates with yomitoki's *own* score
being high, which the ablation panel already established directly and
unconditionally — this cross-tab neither confirms nor refutes whether
that response is *correctly calibrated* against real difficulty.

### Confidence vs. expert disagreement: not measurable on this dataset

Of the 1,533 molecules rated by ≥2 chemists (where `agreement_fraction`
is defined), `overall.confidence` is **1.0 for 1,531 of them (99.9%) and
0.85 for the remaining 2** — essentially zero variance. Consistent with
round 22's earlier finding that confidence is driven almost entirely by
`applicability.stereo_complete`/`stereo_uncheckable`, and MPScore's
molecules (mostly simple, explicitly-drawn precursor candidates) are
nearly all stereo-complete. **No relationship between confidence and
expert disagreement can be assessed on this dataset — reported as "not
measurable," not as a null result**, since a null result would require
actual variance to fail to correlate with.

### Design-change decisions (evidence-based, no implementation this round)

- **A. Fused-aromatic ring burden redesign: SUPPORTED.** The core FM-1
  mechanism (large/ring-rich/highly-aromatic molecules over-penalized)
  replicates independently on a second dataset with a different label
  source, different domain, and negligible molecule overlap. The
  flat-only nuance from TS2/TS3 does not fully replicate (bridging is
  also elevated in MPScore's false positives) — any redesign should
  target ring/atom-count-driven over-penalization broadly, not narrowly
  assume "flat rings only."
- **B. Stereochemical aggregation weight redesign: INCONCLUSIVE.** The
  mechanism is real and directly demonstrated (ablation panel, round 22
  part 2), and its directional signature is present in MPScore's FN
  category, but its *contribution* to MPScore's overall error is small
  relative to the dominant, unexplained false-negative base rate. Not
  enough evidence to prioritize this over addressing the larger,
  unexplained gap first.
- **C. Bridged/cage severity redesign: INCONCLUSIVE — underrepresented.**
  8.9% coverage is too sparse for a dose-response test on this dataset,
  and the available signal (elevated bridging in both TP and FP) doesn't
  separate "correctly detects difficulty" from "over-fires regardless of
  difficulty." Needs a dataset enriched for bridged/caged/macrocyclic
  molecules with independent difficulty labels, which neither TS1/2/3
  nor MPScore provides in quantity.
- **D. Overall confidence redesign: SUPPORTED, but not by this round's
  MPScore data.** The evidence is round 22 part 1's TS1 finding
  (confidence anti-correlated with correctness via a dataset-provenance
  confound) — that finding is unrefuted and this round adds nothing
  either way, since MPScore's confidence values have no variance to
  test. Recorded as supported on the strength of the earlier evidence,
  not double-counted from this round.

### Is a CASP/route-availability-labeled development set needed next? **NO, not yet.**

Reasoning: MPScore's chemist-intuition labels and TS2's Retro*-derived
labels are about as methodologically different as two synthesizability
ground-truth sources can be (subjective expert judgment vs. an automated
retrosynthesis planner's route search; different molecule domains;
different label balance; 0.03% overlap) — and yomitoki scores at chance
level against *both*. If the weakness were specific to Retro*'s
"≤10-step route" semantics, MPScore's independent human-judgment labels
would be the population where yomitoki should have looked *better*, not
identically chance-level. It didn't. That is evidence the weakness is
in yomitoki's own structural-heuristic coverage (a large majority of
"hard" is invisible to all four current components, per the false
-negative analysis above), not in a mismatch between yomitoki's
difficulty concept and any one label source's semantics. Building a
third, CASP-based label source before understanding *why* the existing
two-for-two chance-level result happens would very likely just
reproduce the same pattern a third time at real infrastructure cost
(AiZynthFinder policy/stock setup, compute for route search). The
higher-value next step is root-causing the false-negative dominance
itself — on a development set, per this project's own ordering rule,
never by tuning against TS1/2/3 or re-running this MPScore set
repeatedly until a change looks good on it.

## Part 5 (round 22, part 8): FM-2 interaction diagnosis — does stereo burden separate hard/easy differently for simple vs. complex molecules?

Follow-on round, prompted directly by design candidate B's
"INCONCLUSIVE" verdict above. The ablation panel (round 22 part 2) is a
*controlled, synthetic* demonstration of a mechanism: holding ring count
at 1 and raising stereocenter count 0→4, `stereochemical_burden`'s own
normalized score rises but `overall.difficulty` still can't cross 0.5 —
an aggregation-ceiling effect. That result says nothing about whether
*real* molecules' chemist-judged difficulty actually tracks stereocenter
burden within the population where this ceiling would matter (small,
simple molecules) — a distinct, empirical question this round measures
directly on MPScore. Diagnosis only, per the round's own no-blind-tuning
rule: no weight/threshold/formula is read or changed.
Reproduce: `python3 scripts/evaluate_mpscore.py` (extended this round;
same inputs as Part 4).

**Method**: among MPScore molecules with a defined consensus label and
at least one stereocenter, stratify by two independent "simplicity"
definitions — ring count (≤1 vs. ≥2) and heavy-atom count (median
split, 15 atoms) — and within each stratum compare (a) whether
`stereochemical_burden`'s own contribution differs between chemist-hard
and chemist-easy molecules (bootstrap 95% CI on the mean difference),
(b) that component's own ROC-AUC in isolation, and (c) `overall
.difficulty`'s ROC-AUC in the same stratum, to see whether a real
component-level signal survives aggregation.

### Result: the interaction exists, but not in the shape FM-2 originally implied

| stratum | n (stereo+) | hard / easy | stereo Δ(hard−easy) 95% CI | stereo-only AUC | overall.difficulty AUC |
|---|---|---|---|---|---|
| simple (rings ≤ 1) | 1,894 | 1,674 / 220 | 0.013 [-0.001, 0.028] | 0.520 [0.480, 0.558] | 0.604 [0.562, 0.643] |
| complex (rings ≥ 2) | 2,510 | 2,339 / 171 | **0.131 [0.112, 0.149]** | **0.759 [0.726, 0.792]** | 0.625 [0.570, 0.682] |
| simple (≤ median heavy atoms) | 2,433 | 2,206 / 227 | 0.028 [0.015, 0.042] | 0.554 [0.519, 0.590] | 0.686 [0.655, 0.716] |
| complex (> median heavy atoms) | 1,971 | 1,807 / 164 | **0.137 [0.117, 0.157]** | **0.756 [0.716, 0.790]** | 0.617 [0.563, 0.669] |

(Full set, n=10,543; `fm2_stereo_simplicity_interaction_full` in
`results/mpscore_evaluation.json` has the raw numbers, including the
n_raters≥2 subset — qualitatively the same pattern, smaller n, wider
CIs, see below.)

Two findings, and they cut in different directions from what "FM-2:
boost the stereo weight for small molecules" would predict:

1. **In the *simple* stratum — where the ablation panel's ceiling
   mechanism should matter most — the raw stereo signal itself is weak
   or not statistically distinguishable from noise** (ring-based split:
   CI includes 0; heavy-atom split: barely excludes 0, AUC 0.55, barely
   above chance). This is not what a clean "real signal, suppressed by
   aggregation" story predicts — if the signal doesn't clearly exist in
   the component's own output for this population, boosting its weight
   wouldn't reliably help. **`fraction_predicted_hard_at_threshold` in
   the ring-based simple stratum is 0.0 for both classes** — not one
   molecule, hard-labeled or easy-labeled, crosses `overall.difficulty
   ≥ 0.5` in this stratum. The aggregation ceiling from the ablation
   panel is real and total here, but there may not be much real stereo
   signal underneath it to unlock in this specific population.
2. **In the *complex* stratum, the opposite pattern holds: the stereo
   signal is strong and clean** (AUC ≈ 0.76, a real discriminator) **but
   `overall.difficulty`'s aggregate AUC is markedly worse (≈ 0.62–0.63)
   — meaning the aggregate is actively *destroying* a real signal, not
   just under-weighting a weak one.** The mechanism traces back to
   `ring_topology`, not to stereo being too weak: mean `ring_topology`
   contribution is *higher* for chemist-easy (0.306/0.301) than
   chemist-hard (0.279/0.289) molecules in this exact stratum — the same
   backwards direction Part 4 already found in the full population,
   reproducing here even after conditioning on ring count and stereocenter
   presence. **This links candidate B to candidate A more tightly than
   the two being independent problems**: in the complex-ring population,
   fixing `ring_topology`'s backwards calibration might recover most of
   what looked like a stereo-weighting gap, since it's `ring_topology`'s
   own direction canceling out an otherwise-strong stereo signal, not
   stereo needing to be louder to compete.

The n_raters≥2 subset (n=1,110, cleaner labels but far smaller — as few
as 10-20 "easy" molecules per stratum) shows the same qualitative
pattern with wider CIs; notably the simple-ring stratum's stereo
Δ(hard−easy) becomes significant there (0.046, CI [0.001, 0.092]) where
it wasn't on the full set — consistent with single-rater label noise
diluting a real but modest effect in the full set, though the n is too
small (144 hard / 20 easy) to treat this as a confirmed reversal of
finding 1 above, only as a reason not to treat finding 1 as final either.

### Updated verdict: B is neither confirmed nor cleanly ruled out — it's entangled with A, and MPScore can't resolve it further

- **Where MPScore *can* speak (complex-ring/larger molecules)**: the
  stereo signal is real and strong, and the aggregate is failing to use
  it because of `ring_topology`'s own miscalibration (candidate A's
  mechanism), not because stereo's weight is too small. This is evidence
  *for* fixing A rather than B in this population.
- **Where FM-2 was originally hypothesized to matter most
  (simple/small molecules)**: MPScore's own signal is too weak/noisy to
  confirm the mechanism matters in practice for real molecules, as
  opposed to the controlled ablation panel where it's true by
  construction. This is neither a confirmation nor a refutation of
  candidate B in this population — it's a statement that **MPScore
  cannot resolve this specific question with confidence**: every
  simple-molecule stratum above has only 20-227 "easy"-labeled examples
  against 1,674-2,206 "hard"-labeled ones (MPScore's 84%-hard base rate,
  concentrated further by conditioning on "has a stereocenter"), so any
  tuning decision made against this stratum specifically would be
  fitting noise from a badly imbalanced slice of an already
  not-built-for-tuning dataset (Part 4's own characterization, still
  true here).
- **Explicit answer to "does a development set adequate for *tuning*
  FM-2 exist yet": no.** MPScore diagnoses (this section) but does not
  supply a low-variance quantitative target for what a redesigned
  aggregation should produce on genuinely simple, stereocenter-rich
  molecules. This is the real parked item from this round — not a
  candidate weight value (none was computed or considered), but the
  absence of a dataset that could respect a redesign's own accuracy
  once one is proposed.

### Design-change decision B, updated

**B. Stereochemical aggregation weight redesign: still not ready to
implement, but for a more specific reason than "small contribution."**
The mechanism is real (ablation panel) and the aggregation-ceiling
effect is total in real data too (0% of the simple/ring≤1 stratum
crosses threshold, either class). What's missing is not evidence the
ceiling exists, but evidence of what a *fixed* ceiling should look
like for real simple molecules — and untangling it from candidate A,
since the clearest real-world evidence of a live, usable stereo signal
being lost is in the *complex*-ring population, where the loss traces to
`ring_topology`'s own backwards calibration, not to stereo's weight.
**Recommended framing for future work**: treat A (`ring_topology`
backwards-in-complex-population miscalibration) as the higher-confidence,
better-evidenced target to fix first — it has a clear mechanism, a clear
wrong-direction signature, and fixing it may resolve a meaningful share
of what looked like B's problem for free. B's remaining, distinct
question (does simple/small-molecule stereo burden deserve more weight
on its own terms) stays open and needs either a larger, better-balanced
"easy" sample within the simple/stereocenter-rich population, or a
different ground-truth source entirely — not decided or scoped further
this round.

### Not done in round 22 part 8 (deliberately)

- No weight, threshold, or aggregation-formula value was written,
  computed as a candidate, or even sketched — this section reports
  measurement outcomes only. If a number above reads like it implies a
  specific fix, it doesn't; re-read the "Recommended framing" note.
- No re-measurement against TS1/TS2/TS3 — this section's evidence is
  entirely from MPScore, same discipline as Part 4.
- No attempt to fix `ring_topology`'s backwards-in-complex-population
  direction, despite it being the clearer, better-evidenced finding here
  — flagged as the recommended next target, not started.
- No new labeled dataset built to address the "MPScore can't resolve
  the simple-stratum question" gap identified above — noted as missing,
  not sourced.

## Not done in round 22 parts 2-3 (deliberately)

- No fix. This is diagnosis; per the round's own test-set-integrity and
  no-blind-tuning rules, any actual formula/weight change is a separate,
  later decision, informed by this document but not made here.
- No accuracy re-measurement against TS1/TS2/TS3 — would be pointless
  (nothing changed) and would risk treating a re-look as license to start
  eyeballing test-set numbers again.
- No attempt to build a labeled version of this panel. If a labeled
  development set is needed for actual tuning later, it needs a
  citable, independently-verifiable ground-truth source (e.g. a licensed
  reference corpus, not recalled literature claims) — flagged as
  follow-up, not attempted here.
- The `ring_topology` bridged-complexity saturation (CTRL axis) is
  reported but not chased further — a second, distinct question from
  FM-1/FM-2 that this round's scope didn't extend to.
- No weight/threshold/formula/confidence change, on the strength of
  MPScore's results or otherwise — every design-change candidate above
  is a decision record, not an implementation.
- No re-measurement of TS1/TS2/TS3 — this section's evidence comes
  entirely from MPScore, a genuinely new dataset, not a re-look at the
  confirmatory sets.
- No CASP/AiZynthFinder route-availability set built or started — judged
  not yet warranted, see the reasoning above; may be revisited once the
  false-negative-dominance root cause is better understood.
- `stevenkbennett/synthetic_accessibility_project`'s `training_database.csv`,
  `training_mols.{csv,json}`, and `reaxys_database.csv` were not used —
  only the three raw per-chemist files, since `training_database.csv`'s
  own `chemist_score` column was found not to be a resolved consensus
  (see `../datasets/README.md`).
