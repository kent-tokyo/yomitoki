#!/usr/bin/env python3
"""Chemical-Space Error Atlas, step 5 (round 22 part 12): counterfactual
near-neighbor pair mining.

Finds molecule pairs that are highly similar by Morgan/Tanimoto
similarity (same fingerprint config as embed_umap.py: radius 2, 2048
bits, chirality included) but disagree on expert label or on YOMITOKI's
prediction. These are candidates for "the smallest structural
difference YOMITOKI is missing" -- not proof of a specific mechanism,
just the sharpest available contrast pairs for manual/next-round
chemical inspection.

No scoring code touched. No TS1/TS2/TS3.
"""

import json
import sys

import numpy as np
from rdkit import Chem, DataStructs, RDLogger
from rdkit.Chem import rdFingerprintGenerator

RDLogger.DisableLog("rdApp.*")

from common import RESULTS_DIR, load_features

SIMILARITY_THRESHOLD = 0.7  # standard "highly similar" cut in Morgan/Tanimoto cheminformatics practice
TOP_N_PAIRS = 50
FINGERPRINT_RADIUS = 2
FINGERPRINT_BITS = 2048
FINGERPRINT_INCLUDE_CHIRALITY = True


def main():
    df = load_features().reset_index(drop=True)
    gen = rdFingerprintGenerator.GetMorganGenerator(
        radius=FINGERPRINT_RADIUS, fpSize=FINGERPRINT_BITS, includeChirality=FINGERPRINT_INCLUDE_CHIRALITY
    )
    mols = [Chem.MolFromSmiles(s) for s in df["smiles"]]
    if any(m is None for m in mols):
        raise SystemExit("a canonical SMILES from features.csv failed to re-parse -- investigate")
    fps = [gen.GetFingerprint(m) for m in mols]

    labels = df["expert_label"].to_numpy()
    predicted = df["predicted_hard"].to_numpy()
    ids = df["id"].to_numpy()

    pairs = []
    seen = set()
    for i in range(len(fps)):
        sims = np.array(DataStructs.BulkTanimotoSimilarity(fps[i], fps))
        sims[i] = -1.0  # exclude self
        candidates = np.where(sims >= SIMILARITY_THRESHOLD)[0]
        for j in candidates:
            j = int(j)
            key = (min(i, j), max(i, j))
            if key in seen:
                continue
            label_differs = labels[i] != labels[j]
            prediction_differs = predicted[i] != predicted[j]
            if not (label_differs or prediction_differs):
                continue
            seen.add(key)
            pairs.append(
                {
                    "id_a": ids[i],
                    "id_b": ids[j],
                    "smiles_a": df.loc[i, "smiles"],
                    "smiles_b": df.loc[j, "smiles"],
                    "tanimoto_similarity": float(sims[j]),
                    "expert_label_a": int(labels[i]),
                    "expert_label_b": int(labels[j]),
                    "label_differs": bool(label_differs),
                    "yomitoki_predicted_hard_a": int(predicted[i]),
                    "yomitoki_predicted_hard_b": int(predicted[j]),
                    "prediction_differs": bool(prediction_differs),
                    "yomitoki_difficulty_a": float(df.loc[i, "yomitoki_difficulty"]),
                    "yomitoki_difficulty_b": float(df.loc[j, "yomitoki_difficulty"]),
                    "confusion_a": df.loc[i, "confusion"],
                    "confusion_b": df.loc[j, "confusion"],
                    "stereocenters_a": int(df.loc[i, "stereocenters"]),
                    "stereocenters_b": int(df.loc[j, "stereocenters"]),
                    "ring_count_a": int(df.loc[i, "ring_count"]),
                    "ring_count_b": int(df.loc[j, "ring_count"]),
                }
            )

    # Rank by category, most diagnostic first:
    #   1. expert label differs, YOMITOKI prediction does NOT -- the exact
    #      shape the round's own example describes: YOMITOKI is blind to a
    #      distinction two near-identical molecules' experts actually made.
    #   2. YOMITOKI prediction differs, expert label does NOT -- the
    #      opposite failure: YOMITOKI is *over*-sensitive to a structural
    #      difference experts didn't think mattered.
    #   3. both differ -- both signals moved together; still a real
    #      contrast pair, but less diagnostic than 1/2 since nothing here
    #      contradicts YOMITOKI's own behavior.
    # Within each category, highest similarity first (smallest structural
    # difference is the sharpest contrast).
    def category(p):
        if p["label_differs"] and not p["prediction_differs"]:
            return 0
        if p["prediction_differs"] and not p["label_differs"]:
            return 1
        return 2

    category_names = {0: "label_differs_only", 1: "prediction_differs_only", 2: "both_differ"}
    for p in pairs:
        p["category"] = category_names[category(p)]

    # Report a representative slice of EACH category, not just whichever
    # is most numerous -- label_differs_only pairs vastly outnumber the
    # other two (241 vs 58 total prediction-differing), and a plain
    # global top-N would silently crowd the other, equally diagnostic,
    # categories out entirely.
    per_category_cap = TOP_N_PAIRS // 3
    by_category = {name: [] for name in category_names.values()}
    for p in sorted(pairs, key=lambda p: -p["tanimoto_similarity"]):
        by_category[p["category"]].append(p)
    top_pairs = (
        by_category["label_differs_only"][:per_category_cap]
        + by_category["prediction_differs_only"][:per_category_cap]
        + by_category["both_differ"][:per_category_cap]
    )

    out_path = RESULTS_DIR / "neighbor_pairs.csv"
    import csv

    with open(out_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(top_pairs[0].keys()) if top_pairs else [])
        writer.writeheader()
        writer.writerows(top_pairs)

    n_label_differs = sum(1 for p in pairs if p["label_differs"])
    n_prediction_differs = sum(1 for p in pairs if p["prediction_differs"])
    print(f"wrote {len(top_pairs)} pairs (of {len(pairs)} total >= {SIMILARITY_THRESHOLD} similarity) to {out_path}", file=sys.stderr)
    print(f"total pairs with differing expert label: {n_label_differs}", file=sys.stderr)
    print(f"total pairs with differing YOMITOKI prediction: {n_prediction_differs}", file=sys.stderr)

    with open(RESULTS_DIR / "neighbor_pairs_summary.json", "w") as f:
        json.dump(
            {
                "similarity_threshold": SIMILARITY_THRESHOLD,
                "n_total_pairs_found": len(pairs),
                "n_label_differs": n_label_differs,
                "n_prediction_differs": n_prediction_differs,
                "n_top_pairs_reported": len(top_pairs),
            },
            f,
            indent=2,
        )


if __name__ == "__main__":
    main()
