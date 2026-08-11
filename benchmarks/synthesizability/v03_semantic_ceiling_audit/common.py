"""Shared loaders/constants for the v0.3 Semantic Ceiling Audit (round
22 part 21). Reuses the ALREADY-FROZEN PaRoutes final holdout results
(experiment/v020-final-holdout, commits bac0758/04ead40) verbatim, as
post-hoc semantic analysis, not a reopening -- no new score is computed
on any new molecule population, and nothing here feeds back into a
confirmatory decision. PaRoutes is spent; this round never re-derives
route_steps or touches the frozen evaluation subset's membership.
"""

import json
from pathlib import Path

import numpy as np
import pandas as pd
from rdkit import Chem, RDLogger
from rdkit.Chem.Scaffolds import MurckoScaffold

RDLogger.DisableLog("rdApp.*")

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_DIR = SCRIPT_DIR / "results"
HOLDOUT_DIR = SCRIPT_DIR.parent / "final_holdout"

SEED = 0
N_FOLDS = 5


def load_frozen_holdout():
    """The exact 9,996-molecule frozen evaluation subset (SMILES,
    route_steps, strata) + the frozen v0.2.0-alpha.1 YOMITOKI score --
    verbatim, unmodified, from experiment/v020-final-holdout."""
    subset = pd.read_csv(HOLDOUT_DIR / "results" / "final_evaluation_subset_with_strata.csv")
    ids = [f"HOLDOUT_{i:05d}" for i in range(len(subset))]
    subset["id"] = ids

    yomitoki_scores = {}
    with open(HOLDOUT_DIR / "results" / "holdout_yomitoki_v02alpha1.jsonl") as f:
        for line in f:
            r = json.loads(line)
            if r["error"] is None:
                yomitoki_scores[r["id"]] = r["yomitoki_difficulty"]
    subset["yomitoki_difficulty"] = subset["id"].map(yomitoki_scores)
    assert subset["yomitoki_difficulty"].notna().all()
    return subset


def load_novel_scaffold_ids():
    novel = pd.read_csv(HOLDOUT_DIR / "results" / "novel_scaffold_subset.csv")
    return set(novel["smiles"])


def scaffold(smiles):
    mol = Chem.MolFromSmiles(smiles)
    if mol is None:
        return None
    scaf = MurckoScaffold.GetScaffoldForMol(mol)
    s = Chem.MolToSmiles(scaf)
    return s if s else f"_ACYCLIC_{Chem.MolToSmiles(mol)}"


def build_scaffold_folds(df, n_folds=N_FOLDS, seed=SEED):
    """Fresh Bemis-Murcko scaffold-grouped 5-fold assignment over the
    PaRoutes molecule pool specifically -- MPScore's fold assignment
    doesn't apply to a different molecule population. Diagnostic-probe
    CV only, never a confirmatory re-evaluation."""
    from sklearn.model_selection import GroupKFold

    scaffolds = df["smiles"].map(scaffold)
    assert scaffolds.notna().all()
    gkf = GroupKFold(n_splits=n_folds)
    fold = np.empty(len(df), dtype=int)
    for fold_idx, (_, val_idx) in enumerate(gkf.split(df, groups=scaffolds)):
        fold[val_idx] = fold_idx
    df = df.copy()
    df["scaffold"] = scaffolds
    df["fold"] = fold
    return df
