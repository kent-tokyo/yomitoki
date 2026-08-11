# Final Holdout Manifest (round 22 part 20)

**This file is committed BEFORE any YOMITOKI or comparator score has
been computed on the evaluation subset below.** Everything in it is
fixed as of this commit. Per explicit instruction: if the one-shot
result (Phase 6+) is bad, it is used as-is for the `v0.2.0` decision —
this manifest is not revised afterward except for a documented,
objectively-justified implementation bug (see "Bug correction policy"
below), never for a formula/weight/threshold change or a different
molecule-selection rule.

## 1. Dataset source

- **Paper**: Genheden, S.; Bjerrum, E. "PaRoutes: towards a framework
  for benchmarking retrosynthesis route predictions." *Digital
  Discovery* **2022**, *1*, 527–539.
  [doi:10.1039/D2DD00015F](https://doi.org/10.1039/D2DD00015F)
  (Royal Society of Chemistry, open access).
- **Data archive**: Zenodo record
  [10.5281/zenodo.6275421](https://zenodo.org/records/6275421)
  (Apache 2.0 license, per the `MolecularAI/PaRoutes` GitHub repository).
- **Files used**: `n1-targets.txt`, `n1-stock.txt`, `n1-routes.json`
  (the "n1" test set — one reference route per target; `n5`, the
  alternate 5-routes-per-target set, and all model/template files in the
  same archive are explicitly **not used**).
- **Checksums (SHA-256), computed at download time**:
  ```
  c0d1b48379e1ceb1129fba4bf3773f73f27bdb22bb4d468417e6e404d3210c15  n1-targets.txt
  4b36825ba3395f7f85a69ba82e750b2263768d4384e06f99ff1e9d74cf6d3b0a  n1-stock.txt
  c25a92a8cc0ac06dfdbeb51a902ed0a3d32a38318089aed7973a7ed5c0822974  n1-routes.json
  ```
- **Label definition, exact**: each target's synthesis route is real
  and historically executed — extracted from patent-literature reaction
  records (USPTO-lineage) — not searched for by any retrosynthesis
  planner. `route_steps` = the depth of the longest sequential chain of
  reaction nodes from the target molecule to a purchasable (`in_stock`)
  precursor, computed by `build_labels.py`'s `route_depth()` (convergent
  branches run in parallel, so route length is bounded by the longest
  single branch — the standard retrosynthesis "number of steps"
  definition, not a total reaction-node count). **Every target has a
  known route by construction — there is no "infeasible" class.** This
  is the dataset's central, load-bearing limitation, disclosed here
  before any scoring, not discovered afterward: PaRoutes tests whether
  YOMITOKI's difficulty score tracks real route length *among molecules
  chemists already succeeded at synthesizing*, not whether it can
  separate genuinely-feasible from genuinely-infeasible molecules the
  way TS1–3/MPScore's binary framing does.
- **Original N**: 10,000 target molecules (`n1-routes.json`, one route
  tree per target, all 10,000 target SMILES distinct).
- **Route-length distribution (original 10,000)**: min 2, max 10, mean
  2.86; `{2: 4027, 3: 4124, 4: 1298, 5: 372, 6: 134, 7: 29, 8: 10, 9: 3,
  10: 3}`.

## 2. Leakage audit (Phase 2, run before any scoring)

Three separate overlap measures against every dataset this project has
previously used, per `leakage_audit.py`:

| Prior dataset | N | Exact (isomeric SMILES) | Non-isomeric connectivity | Bemis-Murcko scaffold |
|---|---|---|---|---|
| MPScore | 10,966 | 0 | 1 | 1,868 |
| TS1 | 1,800 | 0 | 0 | 1,590 |
| TS2 | 1,800 | 1 | 1 | 1,932 |
| TS3 | 1,799 | 3 | 3 | 1,859 |

- **Exact-overlap exclusion**: 4 molecules total (1 in TS2, 3 in TS3;
  the one non-isomeric-connectivity hit beyond the exact hits in MPScore
  is a stereoisomer of a different MPScore molecule, not an identity
  match, and is **not** excluded — only true exact-isomeric-SMILES
  matches are excluded, per instruction). 0 unparseable SMILES.
  **Final evaluation subset: 10,000 − 4 = 9,996 molecules.**
- **Scaffold overlap**: reported, not excluded (16–19% across all four
  prior datasets — the same order of magnitude as this project's own
  prior MPScore-vs-TS1–3 scaffold-overlap measurement, ~10%; expected
  recurrence of common ring systems between any two general
  organic-molecule pools, not evidence of leakage by itself).
- **Secondary novel-scaffold subset**: 7,495 of the 9,996 (75.0%) have a
  Bemis-Murcko scaffold absent from all four prior datasets combined —
  saved separately (`results/novel_scaffold_subset.csv`) for an
  additional robustness read, not the primary evaluation population.
- **Ablation-panel molecules** (this project's own controlled-panel
  reference compounds, e.g. pyridine/octane/cyclohexane): not
  separately checked — fewer than 20 total across every prior round,
  all extremely common simple structures not sourced from any external
  corpus; leakage risk against a 10,000-molecule patent-derived holdout
  is structurally negligible. Recorded as a deliberate scope decision,
  not a silent omission.

## 3. Inclusion / exclusion / canonicalization rules

- Canonicalization: RDKit `Chem.MolToSmiles` — isomeric (stereo-aware)
  for exact-overlap detection, non-isomeric for connectivity-overlap
  detection, `MurckoScaffold.GetScaffoldForMol` for scaffold detection
  (acyclic molecules get a unique `_ACYCLIC_<canonical SMILES>` group,
  never pooled into one bucket — the same fix `information_loss_audit`
  applied, reused here).
- Exclusion: unparseable SMILES (0 found) and exact-isomeric overlap
  with MPScore/TS1/TS2/TS3 (4 found) are removed from the final
  evaluation subset. Nothing else is excluded — no outlier trimming, no
  post-hoc population curation.
- **Final evaluation subset: 9,996 molecules**
  (`results/final_evaluation_subset.csv`,
  `results/final_evaluation_subset_with_strata.csv`).

## 4. Primary and secondary endpoints (Phase 4, fixed before any score)

PaRoutes' ground truth is **continuous** (`route_steps`, 2–10), not
binary — per instruction, this fixes the endpoint choice, not a
free pick:

- **Primary endpoint: Spearman rank correlation** (ρ) between YOMITOKI's
  `overall.difficulty` and `route_steps`, on the final evaluation
  subset. Chosen because it only requires the correct *monotonic
  ranking* between difficulty and route length, not a specific
  functional form — the appropriate comparison for a 0–1 continuous
  score against an integer step count. **This is the endpoint the
  Phase 10 decision gate is actually evaluated against.**
- **Secondary endpoints**:
  - Pearson correlation (linear-relationship view, for completeness).
  - A **fixed, mechanically-derived binary view** for comparability with
    SAscore/BR-SAScore's typical reporting style and with Phase 10's
    AUC-flavored gate language: `binary_label_hard = 1` if
    `route_steps > 3` (the median of the final evaluation subset,
    computed now, before any score exists — see
    `results/strata_and_threshold.json`), else `0`. This gives 1,849
    "harder" / 8,147 "easier" (18.5% / 81.5%). ROC-AUC, PR-AUC, balanced
    accuracy, MCC, FP, FN are reported against this fixed threshold as
    **secondary, not primary** — it is a coarsening of a continuous
    label chosen for comparability, not a claim that route-steps>3 vs
    ≤3 is *the* correct easy/hard boundary.
  - **Correlation-strength bands, fixed now** (standard cheminformatics/
    behavioral-science convention, not derived from this holdout's own
    results, not a precise mathematical translation of an AUC threshold):
    ρ ≥ 0.50 = strong, 0.35–0.50 = moderate, 0.20–0.35 = weak, < 0.20 =
    none. Phase 10's verdict is read through these bands primarily, with
    the secondary binary AUC view reported alongside for cross-check —
    both are reported together, and the final call is made holistically
    if they disagree, not by picking whichever looks better.

## 5. Comparator freeze (Phase 5)

| Method | Version / commit | Binary checksum (SHA-256) |
|---|---|---|
| YOMITOKI v0.1.x baseline | tag `v0.1.1`, commit `bd98f80` | `f74679b8318d823627b1f816d6d172c0cd8975596c7e7e1b76dce642f0d0d14c` |
| YOMITOKI v0.2.0-alpha.1 candidate | tag `v0.2.0-alpha.1`, commit `77dc51c` | `47e705b1c73075f6d4a0a5087ebee49eeed0dbcb23ad87732289b9377c29bcd4` |
| SAscore | RDKit `Contrib.SA_Score.sascorer`, RDKit `2025.09.3` | (RDKit-bundled, not separately versioned) |
| BR-SAScore | PyPI `BRSAScore` `0.1.1`, default config (`reaction_from='uspto', buildingblock_from='emolecules'`) | (PyPI package, not separately versioned) |

All four are run on the **exact same 9,996-molecule final evaluation
subset** — no comparator-specific subsetting. Runner scripts:
`../scripts/run_yomitoki.py`, `../scripts/run_sascore.py`,
`../scripts/run_brsascore.py` (all pre-existing, unmodified this round).
Both YOMITOKI binaries built in isolated worktrees from their exact tag
commits, verified zero `src/`/`tests/`/`Cargo.toml` diff between the
`v0.2.0-alpha.1` tag and this branch's own base.

## 6. Bootstrap method

Paired bootstrap, 1,000 resamples, `rng_seed=0`, via this project's
existing `../scripts/metrics.py::paired_bootstrap_diff_ci` — same
methodology used throughout every prior round's production-candidate
evaluation. 95% CI (2.5th/97.5th percentile) for every pairwise
difference in Phase 7.

## 7. Structural strata (Phase 8, fixed now)

Median split (mechanical, on the final evaluation subset, computed
before any score exists):

| Descriptor | Median (split point) |
|---|---|
| Molecular weight | 365.41 |
| Ring count | 3 |
| Aromatic ring count | 2 |
| Stereocenter count | 0 |
| Heteroatom count | 6 |

Per-molecule strata membership saved in
`results/final_evaluation_subset_with_strata.csv`. No new strata may be
defined after opening.

## 8. Success gates (Phase 10, fixed now)

Read primarily through the Spearman ρ bands in §4; the fixed-threshold
binary AUC is reported alongside for the same STRONG GO / GO / MIXED /
NO-GO structure as originally specified with AUC language, translated
here to this dataset's actual endpoint:

- **STRONG GO**: ρ ≥ 0.50, AND clearly better than the `v0.1.1`
  baseline, AND no catastrophic collapse in any structural stratum
  (§7), AND TS1's strong region not meaningfully damaged (Phase 9),
  AND the explanation contract preserved.
- **GO**: ρ roughly 0.35–0.50, clear improvement over `v0.1.1`, no
  major regression.
- **MIXED**: ρ roughly 0.20–0.35, or large instability across strata —
  reported honestly, decision made without returning to MPScore for
  retuning.
- **NO-GO**: ρ < 0.20, or worse than `v0.1.1`, or a serious structural
  regression. `v0.2.0` stable release is withheld; the opened holdout is
  never reused to retry a fix — any future model change is `v0.3`
  development against a new future holdout.

## 9. Bug correction policy

A result may be corrected **only** if: (1) a genuine implementation bug
is found (not a scoring-formula disagreement), (2) the bug is
objectively identifiable as a bug independent of knowing it changed the
result for the better or worse, and (3) both the before and after
numbers are recorded, with the bug's nature and discovery circumstances
disclosed in full. No other reason permits re-opening or re-scoring
this holdout.

## 10. Frozen YOMITOKI scoring (unchanged by this manifest, restated for the record)

Ring-family L2 aggregation (round 22 part 11), `size_topology` formula
including `SIZE_WEIGHT_PER_HETEROATOM = 0.03` (frozen round 22 part 17),
`stereochemical_burden`, `functional_group_liability`, overall
aggregation weights, verdict thresholds, and the confidence formula are
**not modified by any result in this holdout evaluation**, regardless
of outcome.
