#!/usr/bin/env python3
"""Section 3: raw target-only structural descriptors for the PaRoutes
final-holdout molecule pool. Basic/ring-topology/stereo descriptor
functions reused VERBATIM from information_loss_audit/build_features.py
(same validated RDKit primitives, same union-find ring-family logic,
same acyclic-scaffold fix) -- no new chemical perception. FG detail
reuses YOMITOKI's own already-validated Brenk-alert detector (re-parses
the real CLI's full JSON output for raw finding counts, NOT the
post-aggregation fg_contribution score -- that would leak YOMITOKI's
own output into the "target-only representation" being tested). Morgan
fingerprint (r=2, 2048 bit) is RDKit's own standard descriptor.
"""

import json
import subprocess
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
import numpy as np
from rdkit import Chem, RDLogger
from rdkit.Chem import AllChem, rdMolDescriptors
from rdkit.Chem.Scaffolds import MurckoScaffold

RDLogger.DisableLog("rdApp.*")

sys.path.insert(0, ".")
from common import RESULTS_DIR, load_frozen_holdout

YOMITOKI_BIN = Path(__file__).resolve().parents[3] / "target" / "release" / "yomitoki"


def ring_family_stats(mol):
    rings = mol.GetRingInfo().AtomRings()
    if not rings:
        return {"ring_system_count": 0, "largest_family_size": 0}
    parent = list(range(len(rings)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(x, y):
        px, py = find(x), find(y)
        if px != py:
            parent[px] = py

    for i in range(len(rings)):
        for j in range(i + 1, len(rings)):
            if set(rings[i]) & set(rings[j]):
                union(i, j)

    groups = {}
    for i in range(len(rings)):
        root = find(i)
        groups.setdefault(root, set()).update(rings[i])

    sizes = [len(atoms) for atoms in groups.values()]
    return {"ring_system_count": len(groups), "largest_family_size": max(sizes)}


def compute_descriptors(smi):
    mol = Chem.MolFromSmiles(smi)
    heavy = mol.GetNumHeavyAtoms()

    ri = mol.GetRingInfo()
    rings = ri.NumRings()
    ring_sizes = [len(r) for r in ri.AtomRings()]
    max_ring_size = max(ring_sizes) if ring_sizes else 0
    macrocycle_ring_count = sum(1 for s in ring_sizes if s >= 9)
    fused_ring_atom_count = sum(1 for a in mol.GetAtoms() if sum(1 for r in ri.AtomRings() if a.GetIdx() in r) >= 2)
    family = ring_family_stats(mol)
    aromatic_atoms = sum(1 for a in mol.GetAtoms() if a.GetIsAromatic())

    centers = Chem.FindMolChiralCenters(mol, includeUnassigned=True, useLegacyImplementation=False)
    stereocenters = len(centers)
    specified = sum(1 for _, tag in centers if tag != "?")
    unspecified = stereocenters - specified

    return {
        "heavy_atoms": heavy,
        "mol_wt": rdMolDescriptors.CalcExactMolWt(mol),
        "ring_count": rings,
        "ring_system_count": family["ring_system_count"],
        "largest_family_size": family["largest_family_size"],
        "fused_ring_atom_count": fused_ring_atom_count,
        "macrocycle_ring_count": macrocycle_ring_count,
        "max_ring_size": max_ring_size,
        "aromatic_ring_count": rdMolDescriptors.CalcNumAromaticRings(mol),
        "aromatic_atom_fraction": aromatic_atoms / heavy,
        "rotatable_bonds": rdMolDescriptors.CalcNumRotatableBonds(mol),
        "heteroatom_count": rdMolDescriptors.CalcNumHeteroatoms(mol),
        "stereocenters": stereocenters,
        "stereocenter_density": stereocenters / heavy,
        "specified_stereocenters": specified,
        "unspecified_stereocenters": unspecified,
        "unspecified_stereocenter_fraction": (unspecified / stereocenters) if stereocenters else 0.0,
        "bridgeheads": rdMolDescriptors.CalcNumBridgeheadAtoms(mol),
        "spiro_atoms": rdMolDescriptors.CalcNumSpiroAtoms(mol),
        "tpsa": rdMolDescriptors.CalcTPSA(mol),
        "fraction_csp3": rdMolDescriptors.CalcFractionCSP3(mol),
    }


def fg_detail_from_full_report(report):
    findings = report["findings"]
    fg_findings = [f for f in findings if f["code"] == "FUNCTIONAL_GROUP_REACTIVE"]
    alert_types = set()
    max_evidence = 0.0
    for f in fg_findings:
        expl = f["explanation"]
        if "detected: " in expl:
            name = expl.split("detected: ", 1)[1].split(" (Brenk")[0]
            alert_types.add(name)
        val = f.get("evidence", {}).get("value")
        if val is not None:
            max_evidence = max(max_evidence, val)
    fg_dense = any(f["code"] == "FUNCTIONAL_GROUP_DENSE" for f in findings)
    return {"fg_count": len(fg_findings), "fg_alert_type_count": len(alert_types), "fg_dense_finding": int(fg_dense), "fg_max_evidence_value": max_evidence}


def run_yomitoki_full_json(id_to_smiles):
    input_path = RESULTS_DIR / "_full.smi"
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(input_path, "w") as f:
        for mol_id, smi in id_to_smiles.items():
            f.write(f"{smi}\t{mol_id}\n")
    result = subprocess.run([str(YOMITOKI_BIN), "analyze", "--input", str(input_path), "--format", "jsonl"], capture_output=True, text=True)
    out = {}
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        rec = json.loads(line)
        if "error" not in rec:
            out[rec["input"]] = rec["report"]
    input_path.unlink()
    return out


def main():
    holdout = load_frozen_holdout()

    rows = []
    for _, row in holdout.iterrows():
        d = compute_descriptors(row["smiles"])
        d["id"] = row["id"]
        rows.append(d)

    reports = run_yomitoki_full_json({r["id"]: holdout.loc[holdout["id"] == r["id"], "smiles"].iloc[0] for r in rows})
    for r in rows:
        r.update(fg_detail_from_full_report(reports[r["id"]]))

    # Morgan fingerprint (r=2, 2048 bit), saved separately as a dense array.
    fp_matrix = np.zeros((len(holdout), 2048), dtype=np.uint8)
    for i, smi in enumerate(holdout["smiles"]):
        mol = Chem.MolFromSmiles(smi)
        fp = AllChem.GetMorganFingerprintAsBitVect(mol, 2, nBits=2048)
        arr = np.zeros((2048,), dtype=np.uint8)
        Chem.DataStructs.ConvertToNumpyArray(fp, arr)
        fp_matrix[i] = arr
    np.save(RESULTS_DIR / "morgan_fp.npy", fp_matrix)

    import pandas as pd

    desc_df = pd.DataFrame(rows)
    desc_df.to_csv(RESULTS_DIR / "raw_descriptors.csv", index=False)
    print(f"wrote {len(desc_df)} descriptor rows + {fp_matrix.shape} Morgan FP matrix", file=sys.stderr)


if __name__ == "__main__":
    main()
