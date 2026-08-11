#!/usr/bin/env python3
"""Information-Loss Audit, step 6 (round 22 part 13): systematic audit
of same-connectivity, different-stereochemistry molecule groups with
expert label disagreement.

Explicitly NOT proposing (and this script does not compute) an absolute
R/S-based difficulty signal -- global mirror inversion (enantiomers)
should leave route-free intrinsic difficulty unchanged by design;
enantiomer label disagreements are treated as candidate label noise/
rater/context artifacts, not evidence for a scoring feature. Diastereomer
disagreements are the one case future work might reasonably act on, and
even then only via *relative* stereochemical complexity (symmetry, meso-
ness, diastereomer count) -- not configuration-specific R/S identity.

Method: group molecules by their non-isomeric (stereo-stripped) canonical
SMILES -- true structural analogs, not similarity-based neighbors. Within
each group, stereocenters are compared *positionally* via RDKit's
canonical atom ranking (includeChirality=False), which is invariant to
each molecule's own atom numbering -- verified on a synthetic tartaric-
acid RR/SS/meso fixture before trusting it on the real dataset (see
README.md).
"""

import json
import sys
import warnings
from collections import defaultdict
from itertools import combinations

warnings.filterwarnings("ignore")
import pandas as pd
from rdkit import Chem, RDLogger

RDLogger.DisableLog("rdApp.*")

from common import RESULTS_DIR


def stereo_rank_map(smi):
    mol = Chem.MolFromSmiles(smi)
    if mol is None:
        return None, None
    no_stereo = Chem.MolToSmiles(mol, isomericSmiles=False)
    ranks = list(Chem.CanonicalRankAtoms(mol, includeChirality=False))
    centers = Chem.FindMolChiralCenters(mol, includeUnassigned=True, useLegacyImplementation=False)
    rank_to_tag = {ranks[idx]: tag for idx, tag in centers}
    return no_stereo, rank_to_tag


def classify_pair(tags_a, tags_b):
    shared_ranks = set(tags_a) & set(tags_b)
    specified_shared = [r for r in shared_ranks if tags_a[r] != "?" and tags_b[r] != "?"]
    flipped = [r for r in specified_shared if tags_a[r] != tags_b[r]]
    annotation_only = [r for r in shared_ranks if (tags_a[r] == "?") != (tags_b[r] == "?")]
    same_total_centers = len(tags_a) == len(tags_b)

    if not shared_ranks:
        return "no_shared_stereocenters"
    if flipped and len(flipped) == len(specified_shared) and not annotation_only and same_total_centers and specified_shared:
        return "enantiomer_like_full_inversion"
    if flipped and len(flipped) < len(specified_shared):
        return "diastereomer_relative_configuration_differs"
    if not flipped and annotation_only:
        return "stereo_annotation_completeness_only"
    if not flipped and not annotation_only:
        return "identical_stereo_but_grouped"  # shouldn't normally happen if truly identical -- flag for inspection
    return "other_mixed"


def main():
    df = pd.read_csv(RESULTS_DIR / "features_with_folds.csv")

    connectivity_groups = defaultdict(list)
    rank_maps = {}
    for _, row in df.iterrows():
        no_stereo, rank_to_tag = stereo_rank_map(row["smiles"])
        if no_stereo is None:
            continue
        rank_maps[row["id"]] = rank_to_tag
        connectivity_groups[no_stereo].append(row)

    multi_groups = {k: v for k, v in connectivity_groups.items() if len(v) >= 2}
    disagreement_groups = {}
    for connectivity, rows in multi_groups.items():
        labels = {r["expert_label"] for r in rows}
        if len(labels) > 1:
            disagreement_groups[connectivity] = rows

    pair_records = []
    category_counts = defaultdict(int)
    for connectivity, rows in disagreement_groups.items():
        for a, b in combinations(rows, 2):
            if a["expert_label"] == b["expert_label"]:
                continue  # only pairs that actually disagree are the audit's subject
            category = classify_pair(rank_maps[a["id"]], rank_maps[b["id"]])
            category_counts[category] += 1
            pair_records.append(
                {
                    "connectivity": connectivity,
                    "id_a": a["id"],
                    "id_b": b["id"],
                    "smiles_a": a["smiles"],
                    "smiles_b": b["smiles"],
                    "expert_label_a": int(a["expert_label"]),
                    "expert_label_b": int(b["expert_label"]),
                    "n_raters_a": int(a["n_raters"]),
                    "n_raters_b": int(b["n_raters"]),
                    "category": category,
                    "yomitoki_difficulty_a": float(a["yomitoki_difficulty"]),
                    "yomitoki_difficulty_b": float(b["yomitoki_difficulty"]),
                }
            )

    pairs_df = pd.DataFrame(pair_records)
    pairs_df.to_csv(RESULTS_DIR / "stereo_contradiction_pairs.csv", index=False)

    summary = {
        "n_molecules": int(len(df)),
        "n_connectivity_groups_total": len(connectivity_groups),
        "n_connectivity_groups_with_multiple_stereoisomers": len(multi_groups),
        "n_connectivity_groups_with_label_disagreement": len(disagreement_groups),
        "n_disagreeing_pairs": len(pair_records),
        "category_counts": dict(category_counts),
        "single_rater_pairs": int(sum(1 for p in pair_records if p["n_raters_a"] == 1 and p["n_raters_b"] == 1)),
    }
    with open(RESULTS_DIR / "stereo_contradiction_summary.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(json.dumps(summary, indent=2), file=sys.stderr)


if __name__ == "__main__":
    main()
