#!/usr/bin/env python3
"""Chemical-Space Error Atlas, step 3: Morgan fingerprint + UMAP
structural-similarity embedding (round 22 part 12).

Fingerprint implementation: RDKit's `rdFingerprintGenerator` (Morgan/
ECFP), not chematic's -- chosen for consistency with every other script
in this benchmark harness (which is RDKit-based throughout, see
../scripts/evaluate_mpscore.py's own docstring) rather than adding a new
Rust-to-Python bridge for one development-tooling script. Explicitly
recorded here per the round's instruction to state which was used.

Deliberately NOT naive Euclidean PCA on the binary fingerprint bits --
that treats fingerprint bit-vectors as if they were a Euclidean feature
space, which they aren't (a shared "0" bit between two molecules is not
evidence of similarity the way a shared "1" bit is). Morgan fingerprint
-> UMAP with `metric="jaccard"` (Tanimoto-equivalent for binary vectors:
Tanimoto similarity = 1 - Jaccard distance) is the standard, correct
approach; UMAP computes this via approximate nearest neighbors
(pynndescent), not a full O(n^2) pairwise distance matrix.

t-SNE is not used, per the round's instruction (UMAP is the specified
first candidate).

All configuration below is fixed and printed for the reproducibility
record (README.md).
"""

import json
import sys

import numpy as np
import umap
from rdkit import Chem, RDLogger
from rdkit.Chem import rdFingerprintGenerator

RDLogger.DisableLog("rdApp.*")

from common import RESULTS_DIR, SEED, load_features

FINGERPRINT_RADIUS = 2  # ECFP4-equivalent
FINGERPRINT_BITS = 2048
FINGERPRINT_INCLUDE_CHIRALITY = True  # stereocenters are the whole point of this round's FM-2 focus
UMAP_N_NEIGHBORS = 15
UMAP_MIN_DIST = 0.1
UMAP_METRIC = "jaccard"
UMAP_N_COMPONENTS = 2


def compute_fingerprints(smiles_list):
    gen = rdFingerprintGenerator.GetMorganGenerator(
        radius=FINGERPRINT_RADIUS,
        fpSize=FINGERPRINT_BITS,
        includeChirality=FINGERPRINT_INCLUDE_CHIRALITY,
    )
    fps = np.zeros((len(smiles_list), FINGERPRINT_BITS), dtype=np.uint8)
    valid = np.ones(len(smiles_list), dtype=bool)
    for i, smi in enumerate(smiles_list):
        mol = Chem.MolFromSmiles(smi)
        if mol is None:
            valid[i] = False
            continue
        fps[i] = gen.GetFingerprintAsNumPy(mol)
    return fps, valid


def compute_exact_knn(fps, k):
    """Exact (brute-force) Jaccard k-NN, not pynndescent's approximate
    search. Necessary for reproducibility, verified empirically, not
    assumed: `random_state` (and even explicit `n_jobs=1`) were NOT
    sufficient on this dataset at full scale (10,543 points) -- two runs
    with identical settings differed by up to ~24 coordinate units on a
    ~-8..20 range, only 65% mean 15-NN overlap. pynndescent's approximate
    NN-descent search has its own internal randomized local search that
    umap-learn's `random_state` does not fully control at this size.
    Feeding UMAP an exact, precomputed neighbor graph removes that source
    of nondeterminism entirely -- see README.md's reproducibility section
    for the before/after check. ~30s for this dataset size, negligible
    next to UMAP's own optimization step.
    """
    from sklearn.neighbors import NearestNeighbors

    nn = NearestNeighbors(n_neighbors=k + 1, metric="jaccard", algorithm="brute", n_jobs=-1)
    nn.fit(fps.astype(bool))
    distances, indices = nn.kneighbors(fps.astype(bool))
    return indices, distances


def main():
    df = load_features()
    fps, valid = compute_fingerprints(df["smiles"].tolist())
    if not valid.all():
        raise SystemExit(f"{(~valid).sum()} canonical SMILES from features.csv failed to re-parse -- investigate")

    knn_indices, knn_distances = compute_exact_knn(fps, UMAP_N_NEIGHBORS)

    reducer = umap.UMAP(
        n_neighbors=UMAP_N_NEIGHBORS,
        min_dist=UMAP_MIN_DIST,
        metric=UMAP_METRIC,
        n_components=UMAP_N_COMPONENTS,
        random_state=SEED,
        n_jobs=1,
        precomputed_knn=(knn_indices, knn_distances),
        # Default init='spectral' (eigendecomposition of the neighbor
        # graph) was ALSO verified insufficient for reproducibility, even
        # with the exact precomputed kNN graph above: two runs still
        # differed by up to ~23 coordinate units. Root cause: this
        # dataset has genuine duplicate/near-duplicate Morgan fingerprints
        # (see neighbor_pairs.csv -- several pairs at Tanimoto
        # similarity 1.0), which makes the neighbor graph's eigenspace
        # degenerate for repeated/near-zero eigenvalues; ARPACK's
        # iterative solver converges to a different (equally valid) basis
        # each run. `init="random"` sidesteps the eigendecomposition
        # entirely and, combined with the exact kNN graph and
        # random_state above, gives bit-identical output run-to-run --
        # see README.md's reproducibility section for the verification.
        init="random",
    )
    embedding = reducer.fit_transform(fps)

    out = df[["id", "expert_label", "confusion", "yomitoki_difficulty"]].copy()
    out["UMAP1"] = embedding[:, 0]
    out["UMAP2"] = embedding[:, 1]
    out_path = RESULTS_DIR / "umap_coordinates.csv"
    out.to_csv(out_path, index=False)

    config = {
        "fingerprint": {
            "implementation": "RDKit rdFingerprintGenerator (Morgan/ECFP)",
            "radius": FINGERPRINT_RADIUS,
            "bits": FINGERPRINT_BITS,
            "include_chirality": FINGERPRINT_INCLUDE_CHIRALITY,
        },
        "umap": {
            "n_neighbors": UMAP_N_NEIGHBORS,
            "min_dist": UMAP_MIN_DIST,
            "metric": UMAP_METRIC,
            "n_components": UMAP_N_COMPONENTS,
            "random_state": SEED,
            "n_jobs": 1,
            "knn_graph": "exact (brute-force Jaccard), not pynndescent's approximate search -- see compute_exact_knn's docstring",
        },
        "n_molecules": int(len(df)),
    }
    with open(RESULTS_DIR / "umap_config.json", "w") as f:
        json.dump(config, f, indent=2)

    print(f"wrote {out_path} ({len(out)} rows)", file=sys.stderr)
    print(json.dumps(config, indent=2), file=sys.stderr)


if __name__ == "__main__":
    main()
