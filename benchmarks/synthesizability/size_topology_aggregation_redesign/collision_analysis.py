#!/usr/bin/env python3
"""Section 4: collision analysis. If Case G (aggregation loss) holds,
current L1 aggregation should collapse molecules with meaningfully
different MW/rotatable-bond/heteroatom COMPOSITION into the same or
near-identical size_contribution -- direct, inspectable evidence of
scalar-aggregation information loss, distinct from the statistical
probe deltas in discriminator_probe.py.

Method: for every pair of molecules within a small size_contribution
window (near-collision), rank by composition distance (normalized
burden-term differences) -- large composition distance + near-identical
size score + differing expert labels is the direct evidence this
section asks for. O(n^2) on 10,543 molecules is too slow; bucket by
size_contribution first to keep pair search local.
"""

import json

import numpy as np
import pandas as pd

from common import RESULTS_DIR, THRESHOLD, load_dataset

SCORE_WINDOW = 0.003  # near-collision: size_contribution differs by less than this
MIN_COMPOSITION_DISTANCE = 0.06  # burden-term L1 distance, in the same units as size_contribution's own scale

df = load_dataset().reset_index(drop=True)
df["current_prediction"] = (df["overall_difficulty"] >= THRESHOLD).astype(int)

# Bucket by size_contribution into overlapping windows to keep the pair
# search local instead of O(n^2) over all 10,543 molecules.
order = df.sort_values("size_contribution").reset_index(drop=True)
collisions = []
n = len(order)
i = 0
for i in range(n):
    j = i + 1
    while j < n and order.loc[j, "size_contribution"] - order.loc[i, "size_contribution"] <= SCORE_WINDOW:
        a, b = order.loc[i], order.loc[j]
        comp_dist = abs(a["mw_burden"] - b["mw_burden"]) + abs(a["rotatable_burden"] - b["rotatable_burden"]) + abs(a["heteroatom_burden"] - b["heteroatom_burden"])
        if comp_dist >= MIN_COMPOSITION_DISTANCE:
            collisions.append(
                {
                    "id_a": a["id"], "smiles_a": a["smiles"], "id_b": b["id"], "smiles_b": b["smiles"],
                    "size_contribution_a": float(a["size_contribution"]), "size_contribution_b": float(b["size_contribution"]),
                    "size_score_diff": float(abs(a["size_contribution"] - b["size_contribution"])),
                    "mw_burden_a": float(a["mw_burden"]), "mw_burden_b": float(b["mw_burden"]),
                    "rotatable_burden_a": float(a["rotatable_burden"]), "rotatable_burden_b": float(b["rotatable_burden"]),
                    "heteroatom_burden_a": float(a["heteroatom_burden"]), "heteroatom_burden_b": float(b["heteroatom_burden"]),
                    "composition_distance": float(comp_dist),
                    "expert_label_a": int(a["expert_label"]), "expert_label_b": int(b["expert_label"]),
                    "labels_differ": bool(a["expert_label"] != b["expert_label"]),
                    "current_prediction_a": int(a["current_prediction"]), "current_prediction_b": int(b["current_prediction"]),
                }
            )
        j += 1

collisions_df = pd.DataFrame(collisions)
label_differing = collisions_df[collisions_df["labels_differ"]].sort_values("composition_distance", ascending=False)
top = label_differing.head(50)

report = {
    "n_near_collision_pairs_total": len(collisions_df),
    "n_near_collision_pairs_with_differing_labels": len(label_differing),
    "score_window": SCORE_WINDOW,
    "min_composition_distance": MIN_COMPOSITION_DISTANCE,
    "top_examples": top.to_dict("records"),
}

RESULTS_DIR.mkdir(parents=True, exist_ok=True)
with open(RESULTS_DIR / "collision_analysis.json", "w") as f:
    json.dump(report, f, indent=2)

print(f"near-collision pairs (|Δsize_contribution|<{SCORE_WINDOW}, composition_distance>={MIN_COMPOSITION_DISTANCE}): {len(collisions_df)}")
print(f"  of which expert labels differ: {len(label_differing)}")
print("\ntop 10 by composition distance:")
for r in top.head(10).to_dict("records"):
    print(
        f"  {r['id_a']} (mw={r['mw_burden_a']:.3f} rb={r['rotatable_burden_a']:.3f} het={r['heteroatom_burden_a']:.2f} label={r['expert_label_a']} pred={r['current_prediction_a']})"
        f"  vs  {r['id_b']} (mw={r['mw_burden_b']:.3f} rb={r['rotatable_burden_b']:.3f} het={r['heteroatom_burden_b']:.2f} label={r['expert_label_b']} pred={r['current_prediction_b']})"
        f"  size_diff={r['size_score_diff']:.5f} comp_dist={r['composition_distance']:.3f}"
    )
