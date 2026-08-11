#!/usr/bin/env python3
"""Information-Loss Audit, step 1 (round 22 part 13): builds F0-F5
feature sets plus Bemis-Murcko scaffold assignment for scaffold-grouped
CV. See README.md and common.py's FEATURE_SETS for the full column
definitions.

Everything here is either a direct RDKit validated primitive, a simple
derived count/threshold on top of RDKit's own SSSR/ring-perception
output (e.g. ring-*system* count via union-find over shared ring atoms
-- set arithmetic, not a new perception algorithm), or content YOMITOKI
itself already computed and reports (Brenk-alert explanations, for
F5's alert-type diversity -- re-parsed from the real CLI output, not a
new hand-curated chemistry rule). No new chemical perception is added
at this stage, per explicit instruction.

Baseline: current main (post FM-1 L2 + chemical-space-atlas
integration). MPScore development set only.
"""

import csv
import json
import subprocess
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
from rdkit import Chem, RDLogger
from rdkit.Chem import rdMolDescriptors
from rdkit.Chem.Scaffolds import MurckoScaffold

RDLogger.DisableLog("rdApp.*")

from common import BENCH_DIR, DATASETS_DIR, RAW_YOMITOKI_PATH, RESULTS_DIR

YOMITOKI_BIN = BENCH_DIR.parent.parent / "target" / "release" / "yomitoki"


def load_labels():
    return {r["id"]: r for r in csv.DictReader(open(DATASETS_DIR / "mpscore_dev_labels.csv"))}


def load_smiles():
    out = {}
    with open(DATASETS_DIR / "mpscore_dev.smi") as f:
        for line in f:
            smi, mol_id = line.rstrip("\n").split("\t")
            out[mol_id] = smi
    return out


def load_yomitoki_standard():
    out = {}
    with open(RAW_YOMITOKI_PATH) as f:
        for line in f:
            r = json.loads(line)
            if r["error"] is None:
                out[r["id"]] = r
    return out


def run_yomitoki_full_json(smiles_map):
    """Re-runs the real CLI directly (batch mode), keeping the FULL
    per-finding `explanation`/`evidence` -- run_yomitoki.py (the shared
    harness script) only extracts `code`, which is all F0/F1 need but
    not enough for F5's alert-*type* diversity. Not a chemistry change:
    this is the same already-computed, already-shipped CLI output,
    just parsed more fully.
    """
    if not YOMITOKI_BIN.exists():
        raise SystemExit(f"yomitoki binary not found at {YOMITOKI_BIN} -- run: cargo build --release --bin yomitoki")
    input_path = RESULTS_DIR / "_mpscore_dev_full.smi"
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(input_path, "w") as f:
        for mol_id, smi in smiles_map.items():
            f.write(f"{smi}\t{mol_id}\n")

    result = subprocess.run(
        [str(YOMITOKI_BIN), "analyze", "--input", str(input_path), "--format", "jsonl"],
        capture_output=True,
        text=True,
    )
    out = {}
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        rec = json.loads(line)
        mol_id = rec["input"]
        if "error" in rec:
            continue
        out[mol_id] = rec["report"]
    input_path.unlink()
    return out


def ring_family_stats(mol):
    """Union-find over RDKit's own SSSR ring-atom-sets (shared-atom
    connectivity) -- set arithmetic on top of a validated primitive, not
    a new ring-classification algorithm (deliberately NOT attempting to
    reproduce chematic's Simple/Fused/Spiro/Bridged classification,
    which has non-trivial edge-case logic -- see ring_family.rs -- that
    would itself be new chemical perception to re-derive from scratch).
    """
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
    if mol is None:
        return None
    heavy = mol.GetNumHeavyAtoms()
    if heavy == 0:
        return None

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

    # Bemis-Murcko scaffold of an acyclic molecule is an empty string --
    # grouping ALL acyclic molecules under one shared placeholder would
    # lump ~13% of this dataset into a single CV group (they have no
    # ring scaffold to leak in the first place; different acyclic
    # molecules aren't "near-identical" just for lacking a ring). Each
    # acyclic molecule gets a unique group instead, via its own
    # canonical SMILES.
    scaffold = MurckoScaffold.MurckoScaffoldSmiles(mol=mol)
    if not scaffold:
        scaffold = f"_ACYCLIC_{Chem.MolToSmiles(mol)}"

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
        "scaffold_smiles": scaffold,
    }


def fg_detail_from_full_report(report):
    findings = report["findings"]
    fg_findings = [f for f in findings if f["code"] == "FUNCTIONAL_GROUP_REACTIVE"]
    alert_types = set()
    max_evidence = 0.0
    for f in fg_findings:
        # explanation format: "Reactive/unstable functional group detected: <name> (Brenk et al. 2008 structural alert)."
        expl = f["explanation"]
        if "detected: " in expl:
            name = expl.split("detected: ", 1)[1].split(" (Brenk")[0]
            alert_types.add(name)
        val = f.get("evidence", {}).get("value")
        if val is not None:
            max_evidence = max(max_evidence, val)
    fg_dense = any(f["code"] == "FUNCTIONAL_GROUP_DENSE" for f in findings)
    return {
        "fg_count": len(fg_findings),
        "fg_alert_type_count": len(alert_types),
        "fg_dense_finding": int(fg_dense),
        "fg_max_evidence_value": max_evidence,
    }


def canonical_smiles(smi):
    mol = Chem.MolFromSmiles(smi)
    return Chem.MolToSmiles(mol) if mol is not None else None


def main():
    labels = load_labels()
    smiles = load_smiles()
    yomitoki_std = load_yomitoki_standard()

    print("re-running yomitoki CLI directly for full finding detail (F5)...", file=sys.stderr)
    full_reports = run_yomitoki_full_json(smiles)

    rows = []
    for mol_id, row in labels.items():
        if row["hard_consensus"] == "":
            continue
        smi = smiles.get(mol_id)
        y_std = yomitoki_std.get(mol_id)
        full_report = full_reports.get(mol_id)
        if smi is None or y_std is None or full_report is None:
            continue
        canon = canonical_smiles(smi)
        d = compute_descriptors(smi)
        if canon is None or d is None:
            continue

        comps = y_std["yomitoki_components"]
        expert_label = int(row["hard_consensus"])
        difficulty = y_std["yomitoki_difficulty"]
        predicted_hard = difficulty >= 0.5
        if expert_label == 1 and predicted_hard:
            confusion = "TP"
        elif expert_label == 1 and not predicted_hard:
            confusion = "FN"
        elif expert_label == 0 and predicted_hard:
            confusion = "FP"
        else:
            confusion = "TN"

        fg_detail = fg_detail_from_full_report(full_report)
        finding_codes = ";".join(f["code"] for f in full_report["findings"])

        rows.append(
            {
                "id": mol_id,
                "smiles": canon,
                "expert_label": expert_label,
                "n_raters": int(row["n_raters"]),
                "agreement_fraction": float(row["agreement_fraction"]),
                "yomitoki_difficulty": difficulty,
                "predicted_hard": int(predicted_hard),
                "confusion": confusion,
                "ring_contribution": comps["ring_topology"]["contribution"],
                "size_contribution": comps["size_topology"]["contribution"],
                "stereo_contribution": comps["stereochemical_burden"]["contribution"],
                "fg_contribution": comps["functional_group_liability"]["contribution"],
                "finding_codes": finding_codes,
                **d,
                **fg_detail,
            }
        )

    fieldnames = list(rows[0].keys())
    out_path = RESULTS_DIR / "features.csv"
    with open(out_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    n_scaffolds = len({r["scaffold_smiles"] for r in rows})
    print(f"wrote {len(rows)} rows to {out_path}", file=sys.stderr)
    print(f"distinct Bemis-Murcko scaffolds: {n_scaffolds}", file=sys.stderr)


if __name__ == "__main__":
    main()
