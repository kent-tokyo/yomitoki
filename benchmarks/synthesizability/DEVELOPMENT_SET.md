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

## Not done this round (deliberately)

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
