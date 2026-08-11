#!/usr/bin/env python3
"""Chemical-Space Error Atlas, step 1 (round 22 part 12): builds the
per-molecule joined dataset that every other script in this directory
reads. Development tooling only -- no production code, no scoring
change. See README.md for the full pipeline and reproducibility record.

Baseline: the CURRENT v0.2 candidate on `main` (FM-1's L2 ring-family
aggregation, RULESET_VERSION 0.11.0 -- commit 3b4c26c), not the pre-L2
formula. Re-reads `../results/raw/mpscore_yomitoki.jsonl`, which must
already reflect that binary (regenerate via
`../scripts/run_yomitoki.py` against a release build of the current
checkout if in doubt -- this script does not rebuild or re-run yomitoki
itself, to keep dataset assembly and yomitoki execution independently
reproducible/inspectable).

Dataset: MPScore development set only. TS1/TS2/TS3 are not read by any
script in this directory.

Descriptors are computed exclusively from RDKit's own validated
primitives (`rdMolDescriptors`, `Chem.GetRingInfo`) -- no chemistry
(aromaticity, ring perception, stereo assignment) is reimplemented
here. The one derived quantity that has no single built-in RDKit
function, ring *system* (family) count, is a plain union-find over
RDKit's own SSSR ring-atom-sets (shared-atom connectivity), not a new
perception algorithm -- same technique already used earlier this
session's MPScore mechanism checks.
"""

import csv
import hashlib
import json
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
from rdkit import Chem, RDLogger
from rdkit.Chem import rdMolDescriptors

RDLogger.DisableLog("rdApp.*")

SCRIPT_DIR = Path(__file__).resolve().parent
BENCH_DIR = SCRIPT_DIR.parent
DATASETS_DIR = BENCH_DIR / "datasets" / "downloaded" / "mpscore"
RAW_YOMITOKI_PATH = BENCH_DIR / "results" / "raw" / "mpscore_yomitoki.jsonl"
RESULTS_DIR = SCRIPT_DIR / "results"
THRESHOLD = 0.5  # yomitoki's own DIFFICULTY_MODERATE_MAX, same convention as ../scripts/evaluate_mpscore.py

FEATURE_COLUMNS = [
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


def load_labels():
    path = DATASETS_DIR / "mpscore_dev_labels.csv"
    return {r["id"]: r for r in csv.DictReader(open(path))}


def load_smiles():
    out = {}
    with open(DATASETS_DIR / "mpscore_dev.smi") as f:
        for line in f:
            smi, mol_id = line.rstrip("\n").split("\t")
            out[mol_id] = smi
    return out


def load_yomitoki():
    out = {}
    with open(RAW_YOMITOKI_PATH) as f:
        for line in f:
            r = json.loads(line)
            if r["error"] is None:
                out[r["id"]] = r
    return out


def ring_system_count(mol):
    """Number of distinct ring *systems* (families) -- SSSR rings merged
    by shared-atom connectivity (plain union-find over
    `Chem.GetRingInfo().AtomRings()`). Not a new perception algorithm;
    RDKit's own SSSR is the only chemistry input.
    """
    rings = mol.GetRingInfo().AtomRings()
    if not rings:
        return 0
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

    return len({find(i) for i in range(len(rings))})


def compute_descriptors(smi):
    mol = Chem.MolFromSmiles(smi)
    if mol is None:
        return None
    heavy = mol.GetNumHeavyAtoms()
    if heavy == 0:
        return None
    rings = mol.GetRingInfo().NumRings()
    aromatic_atoms = sum(1 for a in mol.GetAtoms() if a.GetIsAromatic())
    stereocenters = len(Chem.FindMolChiralCenters(mol, includeUnassigned=True, useLegacyImplementation=False))
    return {
        "heavy_atoms": heavy,
        "mol_wt": rdMolDescriptors.CalcExactMolWt(mol),
        "ring_count": rings,
        "ring_system_count": ring_system_count(mol),
        "aromatic_ring_count": rdMolDescriptors.CalcNumAromaticRings(mol),
        "aromatic_atom_fraction": aromatic_atoms / heavy,
        "rotatable_bonds": rdMolDescriptors.CalcNumRotatableBonds(mol),
        "heteroatom_count": rdMolDescriptors.CalcNumHeteroatoms(mol),
        "stereocenters": stereocenters,
        "stereocenter_density": stereocenters / heavy,
        "bridgeheads": rdMolDescriptors.CalcNumBridgeheadAtoms(mol),
        "spiro_atoms": rdMolDescriptors.CalcNumSpiroAtoms(mol),
        "tpsa": rdMolDescriptors.CalcTPSA(mol),
        "fraction_csp3": rdMolDescriptors.CalcFractionCSP3(mol),
    }


def canonical_smiles(smi):
    mol = Chem.MolFromSmiles(smi)
    return Chem.MolToSmiles(mol) if mol is not None else None


def build_rows(labels, smiles, yomitoki):
    rows = []
    for mol_id, row in labels.items():
        if row["hard_consensus"] == "":
            continue
        smi = smiles.get(mol_id)
        y_rec = yomitoki.get(mol_id)
        if smi is None or y_rec is None:
            continue
        canon = canonical_smiles(smi)
        d = compute_descriptors(smi)
        if canon is None or d is None:
            continue

        comps = y_rec["yomitoki_components"]
        expert_label = int(row["hard_consensus"])
        difficulty = y_rec["yomitoki_difficulty"]
        predicted_hard = difficulty >= THRESHOLD
        if expert_label == 1 and predicted_hard:
            confusion = "TP"
        elif expert_label == 1 and not predicted_hard:
            confusion = "FN"
        elif expert_label == 0 and predicted_hard:
            confusion = "FP"
        else:
            confusion = "TN"

        fg_count = sum(1 for code in y_rec["yomitoki_findings"] if code == "FUNCTIONAL_GROUP_REACTIVE")

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
                "finding_codes": ";".join(y_rec["yomitoki_findings"]),
                **d,
                "fg_count": fg_count,
            }
        )
    return rows


def main():
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    if not RAW_YOMITOKI_PATH.exists():
        raise SystemExit(
            f"{RAW_YOMITOKI_PATH} not found -- run "
            f"`python3 ../scripts/run_yomitoki.py ../datasets/downloaded/mpscore/mpscore_dev.smi "
            f"../results/raw/mpscore_yomitoki.jsonl` against a release build of the current "
            f"(L2-integrated) checkout first."
        )

    labels = load_labels()
    smiles = load_smiles()
    yomitoki = load_yomitoki()
    rows = build_rows(labels, smiles, yomitoki)

    fieldnames = [
        "id",
        "smiles",
        "expert_label",
        "n_raters",
        "agreement_fraction",
        "yomitoki_difficulty",
        "predicted_hard",
        "confusion",
        "ring_contribution",
        "size_contribution",
        "stereo_contribution",
        "fg_contribution",
        "finding_codes",
        *FEATURE_COLUMNS,
    ]
    out_path = RESULTS_DIR / "features.csv"
    with open(out_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    checksum = hashlib.sha256(out_path.read_bytes()).hexdigest()
    confusion_counts = {}
    for r in rows:
        confusion_counts[r["confusion"]] = confusion_counts.get(r["confusion"], 0) + 1

    print(f"wrote {len(rows)} rows to {out_path}", file=sys.stderr)
    print(f"features.csv sha256: {checksum}", file=sys.stderr)
    print(f"confusion matrix: {confusion_counts}", file=sys.stderr)


if __name__ == "__main__":
    main()
