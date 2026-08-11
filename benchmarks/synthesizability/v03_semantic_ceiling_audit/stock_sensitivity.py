#!/usr/bin/env python3
"""Section 10: stock sensitivity. PaRoutes' route_steps is defined
relative to a fixed "stock" (purchasable-precursor) set -- if
nearest-stock similarity predicts route_steps well, that's direct
evidence route length depends on the available starting-material
universe, not purely on target-intrinsic complexity (a route-free,
single-molecule score could never see the stock definition).
"""

import json
import sys

import numpy as np
from rdkit import Chem, RDLogger
from rdkit.Chem import AllChem
from scipy import stats

RDLogger.DisableLog("rdApp.*")

sys.path.insert(0, ".")
from common import RESULTS_DIR, load_frozen_holdout

HOLDOUT_DIR = RESULTS_DIR.parent.parent / "final_holdout"


def morgan_fp(smi):
    mol = Chem.MolFromSmiles(smi)
    if mol is None:
        return None
    return AllChem.GetMorganFingerprintAsBitVect(mol, 2, nBits=2048)


def main():
    holdout = load_frozen_holdout()
    target_fp = np.load(RESULTS_DIR / "morgan_fp.npy").astype(np.float32)

    with open(HOLDOUT_DIR / "downloaded" / "n1-stock.txt") as f:
        stock_smiles = [line.strip().split()[0] for line in f if line.strip()]
    print(f"n1-stock.txt: {len(stock_smiles)} precursor molecules", file=sys.stderr)

    stock_fps = []
    for smi in stock_smiles:
        fp = morgan_fp(smi)
        if fp is not None:
            arr = np.zeros((2048,), dtype=np.float32)
            Chem.DataStructs.ConvertToNumpyArray(fp, arr)
            stock_fps.append(arr)
    stock_matrix = np.array(stock_fps, dtype=np.float32)
    print(f"parsed stock fingerprints: {stock_matrix.shape}", file=sys.stderr)

    stock_popcount = stock_matrix.sum(axis=1)
    target_popcount = target_fp.sum(axis=1)

    # Chunked to bound memory (9996 x 13646 intersection matrix is
    # ~545M float32 entries if done in one shot -- fine, but chunk
    # anyway for headroom).
    nearest_sim = np.zeros(len(target_fp), dtype=np.float32)
    CHUNK = 1000
    for start in range(0, len(target_fp), CHUNK):
        end = min(start + CHUNK, len(target_fp))
        inter = target_fp[start:end] @ stock_matrix.T
        union = target_popcount[start:end, None] + stock_popcount[None, :] - inter
        tanimoto = np.divide(inter, union, out=np.zeros_like(inter), where=union > 0)
        nearest_sim[start:end] = tanimoto.max(axis=1)
        print(f"  stock similarity: {end}/{len(target_fp)}", file=sys.stderr)

    route_steps = holdout["route_steps"].to_numpy(dtype=float)
    rho, p = stats.spearmanr(route_steps, nearest_sim)
    # also within ring strata
    per_stratum = {}
    for level in ["low", "high"]:
        mask = (holdout["ring_stratum"] == level).to_numpy()
        r, pv = stats.spearmanr(route_steps[mask], nearest_sim[mask])
        per_stratum[level] = {"spearman_rho": float(r), "p": float(pv), "n": int(mask.sum())}

    report = {
        "n_stock_molecules": len(stock_smiles),
        "route_steps_vs_nearest_stock_similarity": {"spearman_rho": float(rho), "p": float(p)},
        "by_ring_stratum": per_stratum,
        "nearest_stock_similarity_distribution": {
            "mean": float(nearest_sim.mean()), "median": float(np.median(nearest_sim)),
            "min": float(nearest_sim.min()), "max": float(nearest_sim.max()),
        },
    }
    with open(RESULTS_DIR / "stock_sensitivity.json", "w") as f:
        json.dump(report, f, indent=2)

    import pandas as pd

    pd.DataFrame({"id": holdout["id"], "nearest_stock_similarity": nearest_sim}).to_csv(RESULTS_DIR / "stock_similarity_per_molecule.csv", index=False)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
