# Information-Loss Audit

Round 22 part 13. **Development tooling, not a YOMITOKI production
feature.** No production Rust code touched, no scoring change, no new
production dependency. MPScore development set only — TS1/TS2/TS3 not
read, no new final holdout opened.

## Purpose

The Chemical-Space Error Atlas (round 22 part 12) found that expert
hard/easy separates weakly-but-positively in raw descriptor PCA space
(silhouette +0.124) yet *negatively* in YOMITOKI's own 4D component
space (-0.059) — meaning the compression from raw structure into
`ring_topology`/`size_topology`/`stereochemical_burden`/
`functional_group_liability` may be discarding real, already-available
signal, not just combining it with poorly-chosen weights.

This round quantifies that directly: **does information exist in
already-available descriptors that the current 4 components don't
capture, and if so, where specifically?** Not a production-ML round —
the question is "does information exist," not "what's the best model."

## 0. Chemical-Space Error Atlas → main

Audited (`src/`/`tests/`/`Cargo.toml` diff against `main`: zero lines)
and integrated via fast-forward merge, `af66b49`. Retained as permanent
development tooling, not user-facing.

## 1. Scaffold-grouped cross-validation

Bemis-Murcko scaffolds (RDKit `MurckoScaffold`), `sklearn.GroupKFold`,
5 folds. **Never a random split.** Acyclic molecules (empty Murcko
scaffold, ~13% of the dataset) each get a unique group instead of one
shared placeholder — lumping all of them into one group would have put
13% of the dataset in a single fold and wasn't a real leakage risk
(different acyclic molecules aren't "near-identical" for lacking a
ring). Verified zero scaffold spans multiple folds (`scaffold_cv.py`,
`results/fold_summary.json`):

| fold | n | hard | easy | hard_frac | scaffolds |
|---|---|---|---|---|---|
| 0 | 2109 | 1747 | 362 | 0.828 | 420 |
| 1 | 2109 | 1889 | 220 | 0.896 | 432 |
| 2 | 2109 | 1878 | 231 | 0.890 | 432 |
| 3 | 2108 | 1744 | 364 | 0.827 | 432 |
| 4 | 2108 | 1902 | 206 | 0.902 | 432 |

## 2. Feature sets (F0-F5)

Every descriptor is either a direct RDKit validated primitive, a simple
derived count/threshold on RDKit's own SSSR output (ring-*system*
count via union-find over shared ring atoms — set arithmetic, not new
ring-classification chemistry; chematic's own Simple/Fused/Spiro/
Bridged classifier was deliberately *not* re-derived, per the explicit
"no new chemical perception" instruction), or content YOMITOKI itself
already computes (Brenk-alert explanations for F5, re-parsed from the
real CLI output, not a new hand-curated rule). See `common.py` for
exact column lists.

- **F0** (4): the current `ring_contribution`/`size_contribution`/
  `stereo_contribution`/`fg_contribution`.
- **F1** (15): raw atlas descriptors (chemical_space's set).
- **F2** (10, ring-detail): adds `largest_family_size`,
  `fused_ring_atom_count`, `macrocycle_ring_count`, `max_ring_size` on
  top of what F1 already has for rings.
- **F3** (5, stereo-detail): `stereocenters`, `stereocenter_density`,
  `specified_stereocenters`, `unspecified_stereocenters`,
  `unspecified_stereocenter_fraction`. **No absolute R/S identity.**
- **F4** (6, size/physicochemical): `heavy_atoms`, `mol_wt`,
  `rotatable_bonds`, `fraction_csp3`, `heteroatom_count`, `tpsa`.
- **F5** (4, FG-detail): `fg_count`, `fg_alert_type_count`,
  `fg_dense_finding`, `fg_max_evidence_value`. **Note**:
  `fg_alert_type_count` turned out to equal `fg_count` for all 10,543
  molecules (chematic's Brenk engine never fires the same alert type
  twice on one molecule) — reported honestly rather than presented as
  richer than it is; F5's only genuinely new content beyond F1 is
  `fg_dense_finding` and `fg_max_evidence_value`.

## 3. Diagnostic probes

- **Model A** (linear): `StandardScaler` + L2-regularized
  `LogisticRegression` (`class_weight="balanced"` sample weights,
  `max_iter=2000`).
- **Model B** (shallow nonlinear): `HistGradientBoostingClassifier`
  (`max_depth=3`, `max_iter=100`, fixed — not tuned). Same balanced
  sample weighting.

Both `random_state=0`. 5-fold scaffold-grouped CV throughout (train on
4 folds, predict the held-out one). Metrics: ROC-AUC, balanced
accuracy, MCC, hard-class PR-AUC, easy-class PR-AUC — fold-level
values saved, not just means (`results/probe_results.json`).

## 4. Headline result

| Feature set | Linear AUC | Nonlinear AUC | Bal. Acc. (linear) | MCC (linear) |
|---|---|---|---|---|
| F0 components | 0.6647 | 0.7677 | 0.6088 | 0.1462 |
| F1 raw descriptors | 0.8301 | 0.8581 | 0.7461 | 0.3634 |
| F2 ring detail | 0.7321 | 0.7867 | 0.6749 | 0.2658 |
| F3 stereo detail | 0.5883 | 0.6182 | 0.5741 | 0.1045 |
| F4 size detail | 0.7658 | 0.8282 | 0.6800 | 0.2547 |
| F5 FG detail | 0.7305 | 0.7422 | 0.6708 | 0.2283 |

**F1 vs. F0, paired fold-wise (same 5 folds, same held-out molecules)**:
linear +0.1654 AUC (p=0.0002), nonlinear +0.0903 AUC (p=0.0015). Both
directions of evidence agree; caveat stated explicitly: n=5 folds only,
low statistical power, read as a strong directional signal not a
significance proof.

**F0's own linear→nonlinear gap (+0.1030, p=0.0002) is the *largest* of
any feature set** — bigger than F1's own linear→nonlinear gap (+0.0280,
p=0.03, only marginal). This means: even with zero new information,
just combining the 4 *existing* numbers non-additively recovers real
signal the current fixed linear weighted sum misses. Full metrics per
feature set (both probes): `results/probe_results.json`.

## 5. Add-one-group-in / leave-one-group-out

`add_drop_analysis.py`. Add-one starts from F0 and adds each detail
group; leave-one-out starts from F1 (grouped into ring/stereo/size/FG
subsets) and removes each group.

| | Add to F0 (linear AUC gain) | Add to F0 (nonlinear gain) | Remove from F1 (linear AUC loss) | Remove from F1 (nonlinear loss) |
|---|---|---|---|---|
| ring | +0.0937 | +0.0494 | -0.0204 | -0.0130 |
| **stereo** | **-0.0020** | **-0.0007** | **-0.0017** | **-0.0004** |
| **size** | **+0.1412** | **+0.0858** | **-0.0623** | **-0.0471** |
| FG | +0.1032 | +0.0480 | -0.0050 | -0.0026 |

**Stereo detail adds essentially nothing beyond what `stereo_
contribution` already captures, in either direction.** `stereochemical_
burden`'s existing design is not where the loss is.

**Size/physicochemical detail is the single largest lever, both ways.**
Traced directly to source: `src/components/size_topology.rs`'s own doc
comment states it is "deliberately narrow: molecular weight and
rotatable-bond count only" — `fraction_csp3`, `heteroatom_count`, and
`tpsa` are not inputs to it at all today, despite F4 (which includes
them) achieving 0.766 linear AUC standalone, second only to full F1.

## 6. FN archetypes

`fn_archetypes.py`. KMeans on standardized F1 descriptors, FN molecules
only (n=7,598), k selected by silhouette over k=3..8. Best k=6,
silhouette=0.205 — above the 0.10 floor, real (if moderate) structure,
not forced:

| archetype | n | share | label |
|---|---|---|---|
| 0 | 2487 | 32.7% | low-stereo, low-ring, low-aromatic (simple/flexible) |
| 1 | 1892 | 24.9% | low-stereo, high-FG-dense, low-saturated |
| 2 | 1462 | 19.2% | low-stereo, high-ring, high-aromatic, low-saturated |
| 3 | 1502 | 19.8% | high-stereo, low-aromatic, low-FG, high-saturated (the "classic FM-2" profile) |
| 4 | 171 | 2.3% | high-stereo, high-ring, low-flexible, high-saturated |
| 5 | 84 | 1.1% | high-stereo, high-ring, high-saturated |

**The classic FM-2 profile (archetype 3, plus the small complex-stereo
archetypes 4/5) accounts for ~23% of all false negatives. The other
~77% is simple/flexible, FG-dense, or flat-aromatic-ring-rich — none of
which is a stereo story.** This is the same conclusion the Chemical-
Space Atlas reached qualitatively (FN is pervasive, not narrowly
localized), now with a precise, data-supported breakdown. Full
per-archetype descriptor means / component scores / dominant
FindingCodes / agreement: `results/fn_archetypes.json`.

## 7. Stereo-isomer contradiction audit

`stereo_contradiction_audit.py`. Groups molecules by non-isomeric
(stereo-stripped) canonical SMILES; within groups with >1 stereoisomer,
finds pairs with expert label disagreement, classifies each pair by
comparing stereocenters *positionally* (via RDKit `CanonicalRankAtoms(
includeChirality=False)`, invariant to each molecule's own atom
numbering — verified correct on a synthetic tartaric-acid RR/SS/meso
fixture before trusting it on real data: RR-vs-SS correctly flags both
centers inverted, RR-vs-meso correctly flags only one).

- 9,721 distinct connectivity groups; 698 have multiple stereoisomers
  present; **95 of those (13.6%) have expert label disagreement**,
  126 disagreeing pairs total.
- **69 diastereomer-type** (relative configuration differs — the
  legitimate candidate for future work).
- **42 enantiomer-like** (full mirror inversion) — per explicit design
  principle, NOT proposed as a scoring signal (route-free intrinsic
  difficulty should be mirror-symmetric); most plausibly label noise,
  rater disagreement, or context/availability effects, not a real
  YOMITOKI blind spot.
- 7 no-shared-stereocenters, 7 annotation-completeness-only, 1 flagged
  edge case.
- **108 of 126 pairs (86%) are single-rater-vs-single-rater** — the
  bulk of this signal is not well-supported by multi-rater agreement;
  read with real caution. The 18 pairs involving a multi-rater molecule
  show the same diastereomer > enantiomer pattern, but n is small.

**Conclusion, per explicit instruction**: do not build an absolute
R/S-based difficulty signal. If a future round pursues this, it should
be framed as *relative* stereochemical complexity (symmetry, meso-ness,
diastereomer count) — not configuration-specific identity. Full pairs:
`results/stereo_contradiction_pairs.csv`.

## Decision-tree interpretation (section 6 of the instruction)

- **Case A (F1 >> F0)**: strongly supported (+0.165 linear / +0.090
  nonlinear AUC, both p<0.01).
- **Case B (F1 ≈ F0 ≈ chance)**: not supported — F1 is far above chance.
- **Case C (F1 nonlinear >> F1 linear >> F0)**: only weakly supported in
  its literal form (F1's own nonlinear gain is small, +0.028) — but a
  *closely related* effect is strong: **F0's own linear→nonlinear gap
  is the largest of any feature set**, meaning interaction/nonlinearity
  matters even among the 4 numbers YOMITOKI already computes.
- **Case D (one subspace far stronger)**: partially present as a
  refinement of A, not a competing primary — F4 (size) is unusually
  strong, F3 (stereo) unusually weak, but this sharpens *where* A's
  loss is concentrated rather than replacing A as the explanation.

## Primary diagnosis

**A — current components discard useful existing information.**
Evidence: F1 vs. F0 gap is large, robust across both probes, and highly
significant given the fold count; add/drop analysis localizes the loss
specifically to size/physicochemical detail (`fraction_csp3`,
`heteroatom_count`, `tpsa` — not currently size_topology inputs) and,
to a lesser extent, ring detail and FG detail; stereo detail is already
essentially fully captured. This is not "missing chemical feature" in
the sense of needing new chemistry (B) — every F1-F5 descriptor is
already a validated, already-computed primitive; the information exists
today and is discarded at compression time. A secondary, real effect
(closely related to C) is present too: interaction/nonlinearity among
the existing 4 components recovers real signal a fixed linear sum
misses, independent of adding any new descriptor.

## Single next production experiment (not implemented this round)

**Enrich `size_topology`'s inputs**: add `fsp3` (Csp3 fraction),
`num_heteroatoms`, and `tpsa` alongside the existing `molecular_weight`/
`rotatable_bond_count` — all three confirmed present as chematic-chem
0.13.0 primitives (`chematic_chem::descriptors::{fsp3, num_heteroatoms,
tpsa}`), so this is buildable without any new chemical perception.

Why this one, not the aggregation-interaction finding (also real and
strong): the interaction/nonlinearity effect (F0's large linear→
nonlinear gap) is genuine and important context for a *later* round, but
redesigning the top-level `overall.difficulty` combination is a bigger,
less-scoped change with more explainability risk. Enriching one
component's inputs with three already-validated, already-available
chematic primitives is the more conservative, more surgical, and more
completely evidenced of the two — it meets every adoption-target
criterion (substantial share of FN implicated across nearly all six
archetypes, not a narrow subclass; scaffold-grouped evidence; chemically
interpretable; route-free; deterministic; explainable as additional
weighted terms, not a black box; buildable from existing chematic
primitives) without also carrying the larger redesign's scope risk.

## Reproducibility

Re-ran `run_probes.py` twice against the same `features_with_folds.csv`
— `probe_results.json` was byte-identical both times (`==` on the
loaded JSON, not just close). All randomized steps (`LogisticRegression`,
`HistGradientBoostingClassifier`, `KMeans`) use `random_state=0`
throughout; unlike the Chemical-Space Atlas's UMAP step, none of
scikit-learn's algorithms used here have the spectral-init-style
nondeterminism that round had to work around.

YOMITOKI commit this audit was built against: the Chemical-Space Atlas
integration commit (`af66b49`, `main`, post-L2 `RULESET_VERSION` 0.11.0).
`features.csv` sha256: `d19b3d3e38a5cfb0f2c5131f51590de64c89204f5ace37a7bda608c7e42e1314`.

Library versions: same pins as `../requirements.txt` /
`../chemical_space/requirements.txt` (numpy 1.26.4, scipy 1.17.1,
pandas 2.2.0, scikit-learn 1.8.0, rdkit 2025.9.3).

## Outputs

- `results/features.csv`, `results/features_with_folds.csv` — large,
  per-molecule, fully regeneratable. **Not committed.**
- `results/probe_results.json`, `results/add_drop_results.json`,
  `results/fn_archetypes.json`, `results/fold_summary.json`,
  `results/stereo_contradiction_summary.json`,
  `results/stereo_contradiction_pairs.csv` — small aggregate evidence.
  **Committed.**

## Pipeline

```
python3 build_features.py     # -> results/features.csv (~30s; re-runs the yomitoki CLI directly for F5 detail)
python3 scaffold_cv.py        # -> results/features_with_folds.csv, results/fold_summary.json
python3 run_probes.py         # -> results/probe_results.json (~10s)
python3 add_drop_analysis.py  # -> results/add_drop_results.json (~15s)
python3 fn_archetypes.py      # -> results/fn_archetypes.json
python3 stereo_contradiction_audit.py  # -> results/stereo_contradiction_*.{csv,json} (~20s)
```

## What this round did NOT do (deliberately)

- **No scoring change of any kind** — no weights, thresholds, ring/
  stereo formulas, new components, ML model, or confidence changes.
  Analysis only.
- **No TS1/TS2/TS3, no new final holdout.** MPScore development data
  only, per standing sequencing (`../../../ROADMAP.md`'s "Explicit
  sequencing" note — the holdout stays closed until the full v0.2.0
  scoring-change set is frozen).
- **No absolute R/S-based difficulty signal** — the stereo-contradiction
  audit's enantiomer findings are explicitly framed as candidate label
  noise, not evidence for a mirror-sensitive scoring feature.
- **Candidate 1 (Bridged/Spiro extent) and the original Candidate 2/3
  framing from the Chemical-Space Atlas round are superseded, not
  pursued this round** — this round's controlled, scaffold-grouped
  evidence points to a different, more general lever (size/
  physicochemical detail) than either of those narrower candidates.
  Bridged/Spiro remains recorded as a real, high-confidence,
  low-population finding (FP=78 vs. FN=7598 — not the shortest path to
  overall discriminative power), not discarded.
