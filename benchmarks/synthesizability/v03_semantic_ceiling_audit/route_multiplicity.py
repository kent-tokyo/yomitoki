#!/usr/bin/env python3
"""Section 11: same-target route multiplicity. PaRoutes n5 provides up
to 5 known routes per target, but n5's target pool overlaps n1's by
only 2,642/10,000 (26%) -- n5 is substantially a DIFFERENT molecule
population, not the same targets with alternate routes. Per section 17
("no new holdout"), this script uses ONLY that 2,642-molecule
intersection with the already-frozen, already-opened n1 evaluation
subset -- the other ~7,358 n5-only molecules are never touched or
scored by anything in this round.

For each overlapping target, n5's own route set (which may itself
contain 1-5 routes) gives an estimate of route_steps variability for
that SAME target -- a direct, if partial, estimate of a route-free
predictor's irreducible ambiguity: Var(route_steps | target).
"""

import json
import sys

import numpy as np
import pandas as pd

sys.path.insert(0, ".")
from common import RESULTS_DIR, load_frozen_holdout
from route_metadata_audit import route_stats

HOLDOUT_DIR = RESULTS_DIR.parent.parent / "final_holdout"


def main():
    holdout = load_frozen_holdout()
    frozen_smiles = set(holdout["smiles"])
    n1_steps = dict(zip(holdout["smiles"], holdout["route_steps"]))

    with open(HOLDOUT_DIR / "downloaded" / "n5-routes.json") as f:
        n5_routes = json.load(f)

    # n5-routes.json: does it have one entry per target (single top route
    # like n1), or nested alternates per target? Inspect structure first.
    sample = n5_routes[0]
    is_list_of_alternatives = isinstance(sample, list)

    overlap_rows = []
    if is_list_of_alternatives:
        for entry in n5_routes:
            if not entry:
                continue
            target_smiles = entry[0]["smiles"]
            if target_smiles not in frozen_smiles:
                continue
            steps_list = [route_stats(r)["longest_path"] for r in entry]
            overlap_rows.append({"smiles": target_smiles, "n1_steps": n1_steps[target_smiles], "n5_steps_list": steps_list})
    else:
        # one route per entry (same shape as n1) -- group by target smiles
        # in case n5 lists multiple entries per target.
        by_target = {}
        for route in n5_routes:
            smi = route["smiles"]
            if smi not in frozen_smiles:
                continue
            by_target.setdefault(smi, []).append(route_stats(route)["longest_path"])
        for smi, steps_list in by_target.items():
            overlap_rows.append({"smiles": smi, "n1_steps": n1_steps[smi], "n5_steps_list": steps_list})

    report = {
        "n1_n5_target_overlap": len(overlap_rows),
        "n1_total_targets": 10000,
        "n5_total_targets": 10000,
        "overlap_fraction": len(overlap_rows) / 10000,
        "scope_restriction": "only the n1/n5 overlap is used -- the ~73% of n5 targets absent from the already-opened n1 evaluation subset are never scored or analyzed by this or any script in this round",
    }

    if overlap_rows:
        within_target_var = [np.var(r["n5_steps_list"]) for r in overlap_rows if len(r["n5_steps_list"]) > 1]
        n1_vs_n5_min_diff = [abs(r["n1_steps"] - min(r["n5_steps_list"])) for r in overlap_rows]
        n_multi_route_targets = sum(1 for r in overlap_rows if len(r["n5_steps_list"]) > 1)
        report["n_targets_with_multiple_n5_routes"] = n_multi_route_targets
        report["mean_within_target_route_step_variance"] = float(np.mean(within_target_var)) if within_target_var else None
        report["mean_abs_diff_n1_vs_best_n5_route"] = float(np.mean(n1_vs_n5_min_diff))
        report["max_abs_diff_n1_vs_best_n5_route"] = float(np.max(n1_vs_n5_min_diff))

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_DIR / "route_multiplicity.json", "w") as f:
        json.dump(report, f, indent=2)
    pd.DataFrame(overlap_rows).to_csv(RESULTS_DIR / "route_multiplicity_detail.csv", index=False)

    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
