#!/usr/bin/env python3
"""Section 8: matched-pair analysis. Structurally near-identical
targets (high Morgan Tanimoto similarity) with substantially different
route_steps -- direct, inspectable evidence for whether route length is
mostly a target-structure property (matched pairs should have similar
route_steps) or substantially route-history/precursor/strategy
-dependent (matched pairs would differ a lot despite near-identical
structure).

Full pairwise Tanimoto via one vectorized matmul (BLAS-accelerated) --
feasible at N=9996, no approximate/sampled search needed.
"""

import json
import sys

import numpy as np
import pandas as pd

sys.path.insert(0, ".")
from common import RESULTS_DIR, load_frozen_holdout

SIMILARITY_THRESHOLD = 0.6
MIN_DELTA = 2


def main():
    holdout = load_frozen_holdout()
    fp = np.load(RESULTS_DIR / "morgan_fp.npy").astype(np.float32)
    n = fp.shape[0]

    popcount = fp.sum(axis=1)  # (n,)
    intersection = fp @ fp.T  # (n, n), BLAS matmul
    union = popcount[:, None] + popcount[None, :] - intersection
    tanimoto = np.divide(intersection, union, out=np.zeros_like(intersection), where=union > 0)

    route_steps = holdout["route_steps"].to_numpy(dtype=float)
    delta = np.abs(route_steps[:, None] - route_steps[None, :])

    iu = np.triu_indices(n, k=1)
    sim_flat = tanimoto[iu]
    delta_flat = delta[iu]

    candidate_mask = (sim_flat >= SIMILARITY_THRESHOLD) & (delta_flat >= MIN_DELTA)
    n_candidates = int(candidate_mask.sum())
    print(f"pairs with Tanimoto>={SIMILARITY_THRESHOLD} and |route_steps delta|>={MIN_DELTA}: {n_candidates}", file=sys.stderr)

    idx_i, idx_j = iu[0][candidate_mask], iu[1][candidate_mask]
    sims = sim_flat[candidate_mask]
    deltas = delta_flat[candidate_mask]

    # rank by delta first (the strongest evidence), similarity as tiebreak
    order = np.lexsort((-sims, -deltas))
    top = order[:50]

    yomitoki = holdout["yomitoki_difficulty"].to_numpy()
    rows = []
    for k in top:
        i, j = idx_i[k], idx_j[k]
        rows.append(
            {
                "smiles_a": holdout["smiles"].iloc[i], "smiles_b": holdout["smiles"].iloc[j],
                "route_steps_a": float(route_steps[i]), "route_steps_b": float(route_steps[j]),
                "route_steps_delta": float(deltas[np.where((idx_i == i) & (idx_j == j))[0][0]]),
                "tanimoto_similarity": float(sims[np.where((idx_i == i) & (idx_j == j))[0][0]]),
                "yomitoki_a": float(yomitoki[i]), "yomitoki_b": float(yomitoki[j]),
                "yomitoki_delta": float(abs(yomitoki[i] - yomitoki[j])),
                "ring_stratum_a": holdout["ring_stratum"].iloc[i], "ring_stratum_b": holdout["ring_stratum"].iloc[j],
            }
        )

    report = {
        "similarity_threshold": SIMILARITY_THRESHOLD,
        "min_route_steps_delta": MIN_DELTA,
        "n_total_pairs_checked": int(len(iu[0])),
        "n_candidate_pairs": n_candidates,
        "top_50": rows,
    }
    with open(RESULTS_DIR / "matched_pair_analysis.json", "w") as f:
        json.dump(report, f, indent=2)

    if rows:
        mean_yomitoki_delta = float(np.mean([r["yomitoki_delta"] for r in rows]))
        mean_route_delta = float(np.mean([r["route_steps_delta"] for r in rows]))
        print(f"n_candidate_pairs (structurally near-identical, route_steps differ by >={MIN_DELTA}): {n_candidates}")
        print(f"top-50 mean route_steps delta: {mean_route_delta:.2f}, mean YOMITOKI difficulty delta: {mean_yomitoki_delta:.4f}")
        for r in rows[:10]:
            print(f"  sim={r['tanimoto_similarity']:.3f}  steps {r['route_steps_a']:.0f}->{r['route_steps_b']:.0f} (Δ{r['route_steps_delta']:.0f})  yomitoki Δ{r['yomitoki_delta']:.3f}")
    else:
        print("no matched pairs found at this threshold")


if __name__ == "__main__":
    main()
