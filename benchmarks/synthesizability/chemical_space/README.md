# Chemical-Space Error Atlas

Round 22 part 12. **Development tooling, not a YOMITOKI production
feature.** Nothing here is reachable from the `yomitoki` crate's public
API and no CLI subcommand was added — see `../../../ROADMAP.md` and
`../DEVELOPMENT_SET.md` for the project's actual development-set
discipline this tooling operates under.

## Purpose

Visualize and quantify where YOMITOKI's false positives and false
negatives concentrate in chemical space, to generate (not test) the
next FM-2 scoring hypothesis. Building PCA/UMAP is not the goal — the
goal is the error-localization evidence they produce. **This atlas is a
hypothesis generator. It does not conclude with a scoring change.** See
"What this round did NOT do" below.

Baseline: the current v0.2 candidate on `main` (FM-1's L2 ring-family
aggregation, `RULESET_VERSION` 0.11.0, commit `3b4c26c`) — not the
pre-L2 formula. Dataset: MPScore development set only. TS1/TS2/TS3 are
not read by any script in this directory.

## Pipeline

```
python3 ../scripts/run_yomitoki.py ../datasets/downloaded/mpscore/mpscore_dev.smi \
    ../results/raw/mpscore_yomitoki.jsonl   # if not already fresh against the current build
python3 build_features.py      # -> results/features.csv
python3 embed_pca.py           # -> results/pca_coordinates.csv, results/pca_summary.json
python3 embed_umap.py          # -> results/umap_coordinates.csv, results/umap_config.json (~60s)
python3 analyze_errors.py      # -> results/error_regions.csv, results/error_atlas_summary.json, plots/*.png
python3 neighbor_analysis.py   # -> results/neighbor_pairs.csv, results/neighbor_pairs_summary.json
python3 component_space.py     # -> results/component_space_summary.json, plots/component_space_pca.png
```

Requires `../requirements.txt` plus `requirements.txt` in this
directory (`umap-learn`, `hdbscan`, `matplotlib`, and umap-learn's own
pinned transitive dependencies).

## A. Descriptor PCA

Interpretable descriptors, computed exclusively from RDKit's own
validated primitives (`rdMolDescriptors`, `Chem.GetRingInfo` /
`FindMolChiralCenters`) — no chemistry (aromaticity, ring perception,
stereo assignment) is reimplemented. The one derived quantity without a
single built-in RDKit function, ring *system* (family) count, is a
plain union-find over RDKit's own SSSR ring-atom-sets (shared-atom
connectivity), not a new perception algorithm.

Full list: `heavy_atoms`, `mol_wt`, `ring_count`, `ring_system_count`,
`aromatic_ring_count`, `aromatic_atom_fraction`, `rotatable_bonds`,
`heteroatom_count`, `stereocenters`, `stereocenter_density`,
`bridgeheads`, `spiro_atoms`, `tpsa`, `fraction_csp3`, `fg_count` (the
last is YOMITOKI's own `FUNCTIONAL_GROUP_REACTIVE` finding count from
its real report output, not a reimplemented Brenk-alert matcher).

Standardized (`StandardScaler`, zero mean / unit variance) before PCA —
required given wildly different native scales (e.g. `mol_wt` ~O(100)
vs. `aromatic_atom_fraction` ~O(1)).

## B. Morgan fingerprint + UMAP

**Fingerprint implementation: RDKit's `rdFingerprintGenerator`**, not
chematic's — chosen for consistency with every other script in this
benchmark harness (RDKit-based throughout) rather than adding a new
Rust-to-Python bridge for one development-tooling script.

Deliberately not naive Euclidean PCA on the binary fingerprint bits —
Morgan fingerprint → UMAP with `metric="jaccard"` (Tanimoto-equivalent
for binary vectors) is used instead, computed via an exact k-NN graph
(see Reproducibility below), not pynndescent's approximate search.
t-SNE was not used, per this round's own instruction.

Frozen config (also recorded in `results/umap_config.json`):
radius=2, bits=2048, chirality included (stereocenters are the whole
point of this round's FM-2 focus), `n_neighbors=15`, `min_dist=0.1`,
`metric="jaccard"`, `random_state=0`, `n_jobs=1`, `init="random"`.

## Reproducibility

Dataset checksum (`results/features.csv`, sha256):
`5d6e4846e32a323738b3c6d3f32873b0810c69eddbc7c3ed4a3891d8c1c488be`

YOMITOKI commit this atlas was built against: `3b4c26c` (`main`,
`RULESET_VERSION` 0.11.0 — the L2 ring-family aggregation integration).

Fixed seed everywhere in this directory: `SEED = 0` (`common.py`).

Library versions (exact pins, `requirements.txt`):
numpy 1.26.4, scipy 1.17.1, pandas 2.2.0, scikit-learn 1.8.0, rdkit
2025.9.3, umap-learn 0.5.12, hdbscan 0.8.44, matplotlib 3.10.8, numba
0.66.0, pynndescent 0.6.0, llvmlite 0.48.0.

**PCA**: ran `embed_pca.py` twice — `pca_coordinates.csv` and
`pca_summary.json` were byte-identical (`diff` clean) both times.
`build_features.py`'s `features.csv` also reproduced byte-identical
(same sha256 both runs) — expected, no randomness involved.

**UMAP**: NOT trivially reproducible with default settings, verified
empirically rather than assumed:
- `random_state=0` alone: two runs differed by up to ~20 coordinate
  units on a ~-8..20 range (mean 15-NN neighbor-set overlap only 65%).
- Adding explicit `n_jobs=1` on top: no improvement (still ~24 units
  max diff). Ruled out thread-level nondeterminism as the cause.
- Feeding UMAP an exact, precomputed k-NN graph (removing
  pynndescent's approximate search entirely): still no improvement
  (~23 units max diff). Ruled out the neighbor search itself.
- Root cause found: default `init="spectral"` performs an
  eigendecomposition of the neighbor graph, and this dataset has
  genuine duplicate/near-duplicate Morgan fingerprints (see
  `neighbor_pairs.csv` — several pairs at Tanimoto similarity exactly
  1.0), which makes that eigenspace degenerate for repeated/near-zero
  eigenvalues; ARPACK's iterative solver converges to a different
  (equally valid) basis each run.
- Fix: `init="random"` (sidesteps the eigendecomposition), combined
  with the exact precomputed k-NN graph, `random_state=0`, and
  `n_jobs=1` above. Ran `embed_umap.py` twice with this final
  configuration: `umap_coordinates.csv` was byte-identical both times
  (`DataFrame.equals` returned `True`; not just "close").

This full investigation (four configurations tried, the failing three
kept in `embed_umap.py`'s comments with their measured diffs) is
recorded here rather than silently landing on the fix, since "UMAP with
a fixed seed" is not automatically reproducible and future readers
re-deriving this pipeline should not have to rediscover that the hard
way.

## Outputs

- `results/features.csv`, `results/pca_coordinates.csv`,
  `results/umap_coordinates.csv` — large, per-molecule, fully
  regeneratable from the fixed seed above. **Not committed** (see
  `.gitignore`).
- `results/error_regions.csv`, `results/neighbor_pairs.csv`,
  `results/*_summary.json`, `results/knn_local_rates_umap.csv` — small
  aggregate evidence. **Committed.**
- `plots/*.png` — **committed.**

## What this round did NOT do (deliberately)

- **No scoring code touched, no weight/threshold/formula read or
  changed.** This is a hypothesis generator; the correct next step for
  anything found here is a controlled ablation on development data,
  not a direct edit informed by a plot.
- **TS1/TS2/TS3 not touched.** No new final holdout opened either —
  see `../../../ROADMAP.md`'s "Explicit sequencing" note: a holdout
  opens only once, after the full v0.2.0 scoring-change set (including
  whatever comes out of this atlas) is frozen.
- **No Renkin overlay.** Future work only, not attempted or wired in
  this round — no Renkin dependency or data pipeline was added.
- **UMAP "islands" are not claimed to be independent real chemical
  classes.** Clustering (HDBSCAN) here is strictly an error-localization
  aid; every claim in `error_regions.csv` is about the *molecules* in a
  region (their own descriptors/FindingCodes), not about the cluster
  boundary being chemically meaningful on its own.
