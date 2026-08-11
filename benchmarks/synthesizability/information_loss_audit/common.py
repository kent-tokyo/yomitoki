"""Shared constants/loaders for the Information-Loss Audit (round 22 part
13). See README.md for the full protocol.
"""

from pathlib import Path

import pandas as pd

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_DIR = SCRIPT_DIR / "results"
BENCH_DIR = SCRIPT_DIR.parent
DATASETS_DIR = BENCH_DIR / "datasets" / "downloaded" / "mpscore"
RAW_YOMITOKI_PATH = BENCH_DIR / "results" / "raw" / "mpscore_yomitoki.jsonl"

SEED = 0
N_FOLDS = 5

# Feature-set column groups (round 22 part 13, section 3 of the
# instruction). F0 is YOMITOKI's own output; F1-F5 are all derived from
# RDKit's validated primitives (or, for F5's alert-type diversity, from
# yomitoki's own already-computed Brenk-alert explanations) -- no new
# chemical perception added at this stage.
F0_COLUMNS = ["ring_contribution", "size_contribution", "stereo_contribution", "fg_contribution"]

F1_COLUMNS = [
    "heavy_atoms",
    "mol_wt",
    "ring_count",
    "ring_system_count",
    "aromatic_ring_count",
    "aromatic_atom_fraction",
    "rotatable_bonds",
    "heteroatom_count",
    "stereocenters",
    "stereocenter_density",
    "bridgeheads",
    "spiro_atoms",
    "tpsa",
    "fraction_csp3",
    "fg_count",
]

F2_COLUMNS = [
    "ring_count",
    "ring_system_count",
    "largest_family_size",
    "fused_ring_atom_count",
    "aromatic_ring_count",
    "aromatic_atom_fraction",
    "bridgeheads",
    "spiro_atoms",
    "macrocycle_ring_count",
    "max_ring_size",
]

F3_COLUMNS = [
    "stereocenters",
    "stereocenter_density",
    "specified_stereocenters",
    "unspecified_stereocenters",
    "unspecified_stereocenter_fraction",
]

F4_COLUMNS = [
    "heavy_atoms",
    "mol_wt",
    "rotatable_bonds",
    "fraction_csp3",
    "heteroatom_count",
    "tpsa",
]

F5_COLUMNS = [
    "fg_count",
    "fg_alert_type_count",
    "fg_dense_finding",
    "fg_max_evidence_value",
]

FEATURE_SETS = {
    "F0_components": F0_COLUMNS,
    "F1_raw_descriptors": F1_COLUMNS,
    "F2_ring_detail": F2_COLUMNS,
    "F3_stereo_detail": F3_COLUMNS,
    "F4_size_detail": F4_COLUMNS,
    "F5_fg_detail": F5_COLUMNS,
}


def load_features():
    return pd.read_csv(RESULTS_DIR / "features.csv")
