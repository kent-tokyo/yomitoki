#!/usr/bin/env python3
"""Phase 8 (strata) + Phase 4 (secondary binarization threshold),
computed mechanically from descriptor quantiles / label quantiles on the
final evaluation subset -- BEFORE any YOMITOKI/comparator score exists.
Nothing here is chosen by looking at how any method performs.
"""

import csv
import json
from pathlib import Path

import numpy as np
from rdkit import Chem, RDLogger
from rdkit.Chem import Descriptors, rdMolDescriptors

RDLogger.DisableLog("rdApp.*")

RESULTS = Path(__file__).resolve().parent / "results"

rows = list(csv.DictReader(open(RESULTS / "final_evaluation_subset.csv")))

descriptors = []
for r in rows:
    mol = Chem.MolFromSmiles(r["smiles"])
    descriptors.append(
        {
            "smiles": r["smiles"],
            "mw": Descriptors.MolWt(mol),
            "ring_count": rdMolDescriptors.CalcNumRings(mol),
            "aromatic_ring_count": rdMolDescriptors.CalcNumAromaticRings(mol),
            "stereocenter_count": len(Chem.FindMolChiralCenters(mol, includeUnassigned=True, useLegacyImplementation=False)),
            "heteroatom_count": sum(1 for a in mol.GetAtoms() if a.GetSymbol() not in ("C", "H")),
        }
    )

mw = np.array([d["mw"] for d in descriptors])
rings = np.array([d["ring_count"] for d in descriptors])
aromatic = np.array([d["aromatic_ring_count"] for d in descriptors])
stereo = np.array([d["stereocenter_count"] for d in descriptors])
het = np.array([d["heteroatom_count"] for d in descriptors])
route_steps = np.array([int(r["route_steps"]) for r in rows])

strata = {
    "mw_median": float(np.median(mw)),
    "ring_count_median": float(np.median(rings)),
    "aromatic_ring_count_median": float(np.median(aromatic)),
    "stereocenter_count_median": float(np.median(stereo)),
    "heteroatom_count_median": float(np.median(het)),
    "route_steps_median": float(np.median(route_steps)),
}

# Secondary binarization threshold (Phase 4's continuous-endpoint
# contingency, secondary AUC-style view only -- Spearman correlation is
# primary): "harder" = route_steps strictly above the median, computed
# on the final evaluation subset, fixed now.
binary_threshold = strata["route_steps_median"]
n_hard = int((route_steps > binary_threshold).sum())
n_easy = int((route_steps <= binary_threshold).sum())

report = {
    "strata_medians": strata,
    "binary_threshold_route_steps_gt": binary_threshold,
    "n_hard_above_threshold": n_hard,
    "n_easy_at_or_below_threshold": n_easy,
    "route_steps_distribution": {int(k): int(v) for k, v in zip(*np.unique(route_steps, return_counts=True))},
}

with open(RESULTS / "strata_and_threshold.json", "w") as f:
    json.dump(report, f, indent=2)

# Persist per-molecule strata membership for use after opening.
with open(RESULTS / "final_evaluation_subset_with_strata.csv", "w", newline="") as f:
    fieldnames = ["smiles", "route_steps", "mw", "ring_count", "aromatic_ring_count", "stereocenter_count", "heteroatom_count", "mw_stratum", "ring_stratum", "aromatic_stratum", "stereo_stratum", "heteroatom_stratum", "binary_label_hard"]
    writer = csv.DictWriter(f, fieldnames=fieldnames)
    writer.writeheader()
    for r, d in zip(rows, descriptors):
        writer.writerow(
            {
                "smiles": r["smiles"],
                "route_steps": r["route_steps"],
                "mw": d["mw"],
                "ring_count": d["ring_count"],
                "aromatic_ring_count": d["aromatic_ring_count"],
                "stereocenter_count": d["stereocenter_count"],
                "heteroatom_count": d["heteroatom_count"],
                "mw_stratum": "high" if d["mw"] > strata["mw_median"] else "low",
                "ring_stratum": "high" if d["ring_count"] > strata["ring_count_median"] else "low",
                "aromatic_stratum": "high" if d["aromatic_ring_count"] > strata["aromatic_ring_count_median"] else "low",
                "stereo_stratum": "high" if d["stereocenter_count"] > strata["stereocenter_count_median"] else "low",
                "heteroatom_stratum": "high" if d["heteroatom_count"] > strata["heteroatom_count_median"] else "low",
                "binary_label_hard": int(int(r["route_steps"]) > binary_threshold),
            }
        )

print(json.dumps(report, indent=2))
