"""Shared constants/loaders for the Size-Topology Information
Decomposition (round 22 part 14). Reuses information_loss_audit's exact
feature table and scaffold-fold assignment -- not regenerated, not
reassigned, so every comparison here is paired against that round's F0
result on the identical 5 folds.
"""

from pathlib import Path

import pandas as pd

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_DIR = SCRIPT_DIR / "results"
PRIOR_ROUND_FEATURES = SCRIPT_DIR.parent / "information_loss_audit" / "results" / "features_with_folds.csv"

SEED = 0
N_FOLDS = 5

F0_COLUMNS = ["ring_contribution", "size_contribution", "stereo_contribution", "fg_contribution"]

# Raw existing-primitive size inputs (already used by size_topology.rs
# today, just as their raw continuous values instead of the component's
# own threshold/saturation-transformed output).
RAW_MW_RB = ["mol_wt", "rotatable_bonds"]

# New, currently-unused-by-size_topology descriptors under test.
NEW_TRIO = ["fraction_csp3", "heteroatom_count", "tpsa"]


def load_features():
    if not PRIOR_ROUND_FEATURES.exists():
        raise SystemExit(
            f"{PRIOR_ROUND_FEATURES} not found -- this round reuses information_loss_audit's "
            f"exact feature table and fold assignment (not regenerated). Re-run that round's "
            f"build_features.py + scaffold_cv.py first if it's genuinely missing."
        )
    return pd.read_csv(PRIOR_ROUND_FEATURES)
