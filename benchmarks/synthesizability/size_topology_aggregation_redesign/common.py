"""Shared constants/loaders for the Final Size-Topology Aggregation
Experiment (round 22 part 19). Baseline is 0.2.0-alpha.1; every frozen
constant below is read-only for this round -- MW weight, RB weight,
SIZE_WEIGHT_PER_HETEROATOM (frozen round 22 part 17), the aggregate
overall weights, and SIZE_BURDEN_SCALE (the saturation transform's `/2`)
are none of them touched by anything this round tests. The only axis
under test is how the three existing burden terms (mw/rotatable/
heteroatom) combine into `raw` before the (fixed) saturation transform.
"""

import csv
import json
from pathlib import Path

import numpy as np
import pandas as pd
from rdkit import RDLogger

RDLogger.DisableLog("rdApp.*")

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_DIR = SCRIPT_DIR / "results"
BENCH_DIR = SCRIPT_DIR.parent
DATASETS_DIR = BENCH_DIR / "datasets" / "downloaded" / "mpscore"
PRIOR_ROUND_FEATURES = BENCH_DIR / "information_loss_audit" / "results" / "features_with_folds.csv"
ALPHA1_YOMITOKI_OUTPUT = Path("/tmp/mpscore_yomitoki_alpha1.jsonl")

SEED = 0
N_FOLDS = 5
THRESHOLD = 0.5

# Frozen (round 22 part 17 / 0.2.0-alpha.1) -- read-only this round.
SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT = 0.0006
SIZE_WEIGHT_PER_ROTATABLE_BOND = 0.03
SIZE_WEIGHT_PER_HETEROATOM = 0.03
SIZE_BURDEN_SCALE = 2.0
AGGREGATE_WEIGHT_RING_TOPOLOGY = 1.0
AGGREGATE_WEIGHT_SIZE_TOPOLOGY = 0.4
AGGREGATE_WEIGHT_STEREOCHEMICAL_BURDEN = 0.5
AGGREGATE_WEIGHT_FUNCTIONAL_GROUP_LIABILITY = 0.4

F0_COLUMNS = ["ring_contribution", "size_contribution", "stereo_contribution", "fg_contribution"]


def load_labels():
    return {r["id"]: r for r in csv.DictReader(open(DATASETS_DIR / "mpscore_dev_labels.csv"))}


def load_alpha1_components():
    out = {}
    with open(ALPHA1_YOMITOKI_OUTPUT) as f:
        for line in f:
            r = json.loads(line)
            if r["error"] is None:
                out[r["id"]] = r
    return out


def load_dataset():
    """One row per labeled MPScore molecule: expert label, scaffold fold
    (reused verbatim), raw mol_wt/rotatable_bonds/heteroatom_count, the
    three burden terms computed from them under the FROZEN weights, F0
    (the four current 0.2.0-alpha.1 component contributions, from a real
    binary run -- not recomputed), and overall_difficulty as currently
    reported.
    """
    labels = load_labels()
    alpha1 = load_alpha1_components()
    prior = pd.read_csv(PRIOR_ROUND_FEATURES).set_index("id")

    rows = []
    for mol_id, row in labels.items():
        if row["hard_consensus"] == "" or mol_id not in alpha1 or mol_id not in prior.index:
            continue
        p = prior.loc[mol_id]
        a = alpha1[mol_id]
        comps = a["yomitoki_components"]
        rows.append(
            {
                "id": mol_id,
                "smiles": p["smiles"],
                "expert_label": int(row["hard_consensus"]),
                "fold": int(p["fold"]),
                "mol_wt": float(p["mol_wt"]),
                "rotatable_bonds": int(p["rotatable_bonds"]),
                "heteroatom_count": int(p["heteroatom_count"]),
                "ring_contribution": comps["ring_topology"]["contribution"],
                "stereo_contribution": comps["stereochemical_burden"]["contribution"],
                "fg_contribution": comps["functional_group_liability"]["contribution"],
                "size_contribution": comps["size_topology"]["contribution"],
                "overall_difficulty": a["yomitoki_difficulty"],
            }
        )
    df = pd.DataFrame(rows)
    df["mw_burden"] = SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT * df["mol_wt"]
    df["heteroatom_burden"] = SIZE_WEIGHT_PER_HETEROATOM * df["heteroatom_count"]

    # Data-quality correction (found via this round's section-1 invertibility
    # check, see verify_invertibility.py): the `rotatable_bonds` column in
    # information_loss_audit's cached CSV is RDKit's CalcNumRotatableBonds,
    # which disagrees with chematic's actual rotatable_bond_count() for 91
    # molecules (0.86%) -- mostly S-S/C(=S)/amidine-type bonds RDKit's
    # definition excludes but chematic's includes, up to 3 bonds' worth.
    # Backed out here as the residual after subtracting mw/heteroatom burden
    # from the REAL binary's own size_contribution (inverted through the
    # saturation transform), rounded to the nearest integer bond count --
    # reconstructs real size_contribution to <0.004 max abs diff (vs. 0.090
    # using the uncorrected column), the remainder being the already
    # -documented ~0.22%-relative mol_wt monoisotopic/average-mass mismatch.
    raw_from_report = desaturate(df["size_contribution"])
    residual_rotatable_burden = raw_from_report - df["mw_burden"] - df["heteroatom_burden"]
    corrected_rotatable_bonds = (residual_rotatable_burden / SIZE_WEIGHT_PER_ROTATABLE_BOND).round().clip(lower=0)
    n_corrected = int((corrected_rotatable_bonds != df["rotatable_bonds"]).sum())
    if n_corrected:
        print(f"[common.load_dataset] corrected rotatable_bonds for {n_corrected} molecules (RDKit/chematic definition mismatch)")
    df["rotatable_bonds"] = corrected_rotatable_bonds.astype(int)
    df["rotatable_burden"] = SIZE_WEIGHT_PER_ROTATABLE_BOND * df["rotatable_bonds"]
    return df


def saturate(raw):
    return 1.0 - np.exp(-raw / SIZE_BURDEN_SCALE)


def desaturate(size_contribution):
    """Inverse of `saturate`: raw = -scale * ln(1 - s). Only valid for
    s < 1 (never exactly 1 in practice -- the transform is asymptotic)."""
    return -SIZE_BURDEN_SCALE * np.log(1.0 - size_contribution)


def aggregate_l1(m, r, h):
    return m + r + h


def aggregate_l2(m, r, h):
    return np.sqrt(m**2 + r**2 + h**2)


def aggregate_max(m, r, h):
    return np.maximum(np.maximum(m, r), h)


AGGREGATORS = {"L1_current": aggregate_l1, "L2": aggregate_l2, "MAX": aggregate_max}


def overall_difficulty(ring_c, size_c, stereo_c, fg_c):
    raw = (
        AGGREGATE_WEIGHT_RING_TOPOLOGY * ring_c
        + AGGREGATE_WEIGHT_SIZE_TOPOLOGY * size_c
        + AGGREGATE_WEIGHT_STEREOCHEMICAL_BURDEN * stereo_c
        + AGGREGATE_WEIGHT_FUNCTIONAL_GROUP_LIABILITY * fg_c
    )
    return np.clip(raw, 0.0, 1.0)


def classification_report(y_true, y_score, threshold=THRESHOLD):
    from sklearn.metrics import average_precision_score, balanced_accuracy_score, matthews_corrcoef, roc_auc_score

    y_true = np.asarray(y_true)
    y_score = np.asarray(y_score)
    y_pred = (y_score >= threshold).astype(int)
    n_hard, n_easy = int((y_true == 1).sum()), int((y_true == 0).sum())
    fp = int(((y_true == 0) & (y_pred == 1)).sum())
    fn = int(((y_true == 1) & (y_pred == 0)).sum())
    return {
        "roc_auc": float(roc_auc_score(y_true, y_score)) if n_hard and n_easy else None,
        "hard_class_pr_auc": float(average_precision_score(y_true, y_score)) if n_hard and n_easy else None,
        "balanced_accuracy": float(balanced_accuracy_score(y_true, y_pred)),
        "mcc": float(matthews_corrcoef(y_true, y_pred)) if n_hard and n_easy else None,
        "fp": fp,
        "fn": fn,
        "n_hard": n_hard,
        "n_easy": n_easy,
    }
