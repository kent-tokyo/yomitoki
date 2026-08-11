#!/usr/bin/env python3
"""Follow-up to the Semantic Ceiling Audit's own recommended next
experiment (README.md 'Next experiment'): what does the Morgan
fingerprint probe see in the high-ring regime that YOMITOKI's current
four components don't? Post-hoc analysis of the already-frozen PaRoutes
holdout -- no new molecule population, no scoring change, zero
production diff.

Method: fit the same nonlinear model (HistGradientBoostingRegressor)
used in predictability_probes.py on the Morgan fingerprint, restricted
to the high-ring subset, then use permutation importance (not
impurity-based -- HistGradientBoostingRegressor doesn't expose that,
and permutation importance is more reliable anyway) to find which bits
matter most. Map the top bits back to their actual substructure
environment via RDKit's own bitInfo (the standard, validated way to
interpret a Morgan bit -- not a new hand-rolled substructure detector).
"""

import json
import sys
import warnings

import numpy as np
import pandas as pd
from rdkit import Chem, RDLogger
from rdkit.Chem import AllChem
from sklearn.ensemble import HistGradientBoostingRegressor
from sklearn.inspection import permutation_importance

warnings.filterwarnings("ignore")
RDLogger.DisableLog("rdApp.*")

sys.path.insert(0, ".")
from common import RESULTS_DIR, SEED, load_frozen_holdout

N_TOP_BITS = 20


def main():
    holdout = load_frozen_holdout()
    high_ring = holdout[holdout["ring_stratum"] == "high"].reset_index(drop=True)
    print(f"high-ring subset: {len(high_ring)} molecules", file=sys.stderr)

    fp_matrix = np.zeros((len(high_ring), 2048), dtype=np.uint8)
    bit_info_by_mol = []
    mols = []
    for i, smi in enumerate(high_ring["smiles"]):
        mol = Chem.MolFromSmiles(smi)
        mols.append(mol)
        bit_info = {}
        fp = AllChem.GetMorganFingerprintAsBitVect(mol, 2, nBits=2048, bitInfo=bit_info)
        arr = np.zeros((2048,), dtype=np.uint8)
        Chem.DataStructs.ConvertToNumpyArray(fp, arr)
        fp_matrix[i] = arr
        bit_info_by_mol.append(bit_info)

    y = high_ring["route_steps"].to_numpy(dtype=float)
    X = fp_matrix.astype(float)

    # Single fit on all high-ring data (interpretability exercise, not a
    # performance claim -- predictability_probes.py already established
    # the CV'd performance number, rho=0.130, for this same population).
    model = HistGradientBoostingRegressor(max_depth=4, max_iter=150, random_state=SEED)
    model.fit(X, y)

    print("running permutation importance...", file=sys.stderr)
    perm = permutation_importance(model, X, y, n_repeats=10, random_state=SEED, scoring="r2", n_jobs=-1)
    importances = perm.importances_mean
    top_bits = np.argsort(-importances)[:N_TOP_BITS]

    bit_reports = []
    for bit in top_bits:
        if importances[bit] <= 0:
            continue
        # Find example (molecule, atom, radius) environments for this bit
        # across the high-ring subset -- RDKit's own bitInfo, not a new
        # perception rule.
        examples = []
        for mol_idx, bi in enumerate(bit_info_by_mol):
            if int(bit) in bi:
                for atom_idx, radius in bi[int(bit)][:1]:  # one example env per molecule is enough
                    examples.append({"mol_idx": mol_idx, "smiles": high_ring["smiles"].iloc[mol_idx], "atom_idx": atom_idx, "radius": radius})
                if len(examples) >= 3:
                    break

        env_smarts = []
        for ex in examples[:3]:
            mol = mols[ex["mol_idx"]]
            if ex["radius"] == 0:
                env_smarts.append(mol.GetAtomWithIdx(ex["atom_idx"]).GetSymbol())
                continue
            env = Chem.FindAtomEnvironmentOfRadiusN(mol, ex["radius"], ex["atom_idx"])
            amap = {}
            submol = Chem.PathToSubmol(mol, env, atomMap=amap)
            env_smarts.append(Chem.MolToSmiles(submol) if submol.GetNumAtoms() else None)

        n_high_ring_mols_with_bit = int(fp_matrix[:, bit].sum())
        bit_reports.append(
            {
                "bit": int(bit),
                "permutation_importance": float(importances[bit]),
                "n_high_ring_molecules_with_bit": n_high_ring_mols_with_bit,
                "fraction_high_ring": n_high_ring_mols_with_bit / len(high_ring),
                "example_environments": env_smarts,
                "example_smiles": [ex["smiles"] for ex in examples[:3]],
            }
        )

    report = {
        "n_high_ring": len(high_ring),
        "model_r2_on_training_data": float(model.score(X, y)),
        "top_bits": bit_reports,
    }
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_DIR / "morgan_substructure_audit.json", "w") as f:
        json.dump(report, f, indent=2)

    print(f"\nmodel R^2 on high-ring training data (in-sample, not a CV performance claim): {report['model_r2_on_training_data']:.4f}")
    print(f"\ntop {len(bit_reports)} Morgan bits by permutation importance:")
    for b in bit_reports:
        print(f"  bit={b['bit']:5d}  importance={b['permutation_importance']:.5f}  n_mols={b['n_high_ring_molecules_with_bit']} ({b['fraction_high_ring']*100:.1f}%)  env_example={b['example_environments'][0]}")


if __name__ == "__main__":
    main()
