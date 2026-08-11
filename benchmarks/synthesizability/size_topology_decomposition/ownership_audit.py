#!/usr/bin/env python3
"""Size-Topology Information Decomposition, step 4 (round 22 part 14):
semantic ownership + double-counting audit.

Predictive power alone does not imply a descriptor belongs in
size_topology -- heteroatom_count and tpsa are chemically adjacent to
functional-group burden (most Brenk reactive-group alerts involve N/O/S/
halogen atoms), so their apparent size_topology-relevant signal could
just be a redundant re-statement of what functional_group_liability
already penalizes. Checked directly via correlation AND via whether the
signal survives once fg_contribution is already in the baseline (it is,
in every S1-S6 comparison in decompose.py -- F0 always includes
fg_contribution).
"""

import json
import sys
import warnings

import numpy as np

warnings.filterwarnings("ignore")

from common import load_features


def main():
    df = load_features()

    correlations = {}
    pairs = [
        ("heteroatom_count", "fg_count"),
        ("heteroatom_count", "fg_contribution"),
        ("tpsa", "fg_count"),
        ("tpsa", "fg_contribution"),
        ("fraction_csp3", "fg_count"),
        ("fraction_csp3", "ring_contribution"),
        ("fraction_csp3", "stereocenters"),
        ("fraction_csp3", "stereo_contribution"),
        ("heteroatom_count", "tpsa"),  # these two are almost definitionally related
        ("heteroatom_count", "mol_wt"),
        ("tpsa", "mol_wt"),
    ]
    for a, b in pairs:
        r = float(np.corrcoef(df[a], df[b])[0, 1])
        correlations[f"{a}_vs_{b}"] = r

    print("Correlations:", file=sys.stderr)
    for k, v in correlations.items():
        print(f"  {k}: r={v:.3f}", file=sys.stderr)

    # Semantic ownership judgment -- recorded as a structured decision,
    # not just prose, so it's auditable.
    ownership = {
        "fraction_csp3": {
            "candidates_considered": ["size/topology", "ring/topological complexity", "new structural-dimensionality evidence"],
            "correlation_with_ring_contribution": correlations["fraction_csp3_vs_ring_contribution"],
            "correlation_with_stereo_contribution": correlations["fraction_csp3_vs_stereo_contribution"],
            "conditional_significance_after_MW_RB": "NOT significant (p=0.23, decompose.py S1_plus_fsp3)",
            "judgment": (
                "Not independently informative once MW/RB are present (this round's own "
                "conditional test) -- moot for ownership since it doesn't clear the bar to "
                "add anywhere. If it had, low correlation with ring_contribution/stereo_"
                "contribution would have suggested 'new structural-dimensionality evidence' "
                "over 'ring/topological complexity', but this is not being pursued."
            ),
        },
        "heteroatom_count": {
            "candidates_considered": ["size/composition", "functional-group burden", "neither directly"],
            "correlation_with_fg_count": correlations["heteroatom_count_vs_fg_count"],
            "correlation_with_fg_contribution": correlations["heteroatom_count_vs_fg_contribution"],
            "correlation_with_mol_wt": correlations["heteroatom_count_vs_mol_wt"],
            "conditional_significance_after_MW_RB": "significant (p=0.022, decompose.py S1_plus_heteroatoms)",
            "conditional_significance_after_fg_contribution": (
                "F0 (baseline for every S1-S6 comparison) already includes fg_contribution -- "
                "heteroatom_count's standalone significance (S3 vs S0, p=0.012) is therefore "
                "ALREADY measured on top of the existing FG signal, not instead of it."
            ),
            "judgment": "size/composition -- see main text for the full reasoning.",
        },
        "tpsa": {
            "candidates_considered": ["physicochemical/composition", "functional-group-related evidence", "neither directly"],
            "correlation_with_fg_count": correlations["tpsa_vs_fg_count"],
            "correlation_with_fg_contribution": correlations["tpsa_vs_fg_contribution"],
            "correlation_with_heteroatom_count": correlations["heteroatom_count_vs_tpsa"],
            "conditional_significance_after_MW_RB": "NOT significant (p=0.39, decompose.py S1_plus_tpsa)",
            "judgment": "Not independently informative -- moot for ownership, not pursued.",
        },
    }

    report = {"correlations": correlations, "semantic_ownership": ownership}
    with open("results/ownership_audit.json", "w") as f:
        json.dump(report, f, indent=2)
    print("\nwrote results/ownership_audit.json", file=sys.stderr)


if __name__ == "__main__":
    main()
