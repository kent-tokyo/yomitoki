#!/usr/bin/env python3
"""Downloads and processes the MPScore expert-chemist development set.

Provenance (verified directly against the downloaded files -- see
benchmarks/synthesizability/datasets/README.md's "MPScore" section for
the full writeup):

  - Source: stevenkbennett/synthetic_accessibility_project, MIT license.
    Pinned to release tag 1.0.6 (commit b8f4d313c41dbdef1f3404b52d83e
    9a71673c581), not the moving `revisions` branch tip.
  - Three raw per-chemist files, inconsistent format (not normalized
    upstream): filip.csv (SMILES-keyed, tab-delimited), opinions_becky
    .json and opinions_mebriggs.json (InChI-keyed dicts).
  - `data/training_database.csv`'s `chemist_score` column is NOT used as
    ground truth -- verified to be one row per rating event, not per
    molecule (1,544 duplicate-molecule row groups, 435 of them
    internally conflicting). Consensus is computed HERE, independently,
    from the three raw per-chemist files: majority vote where >=2 raters
    agree, `None` (undefined) where exactly 2 raters split 1-1, the
    single rater's vote where only one rater exists. `n_raters` and
    `agreement_fraction` are preserved per molecule, not collapsed away.
  - Label direction is the OPPOSITE of BR-SAScore's TS1/2/3: the source
    data's 1 = easy-to-synthesize, 0 = difficult. This script flips it
    once, explicitly, to this project's hard=1 convention (matching
    BR-SAScore's `labels`) -- `hard_*` columns in the output, never the
    source's raw 1=easy meaning left ambiguous in a column that doesn't
    say so.
  - Exact-molecule overlap with TS1/TS2/TS3 is 3/10969 molecules --
    excluded from the output by default (see EXACT_TS_OVERLAP below).
"""

import argparse
import csv
import hashlib
import json
import subprocess
import sys
import warnings
from collections import Counter
from pathlib import Path

warnings.filterwarnings("ignore")
from rdkit import Chem, RDLogger  # noqa: E402

RDLogger.DisableLog("rdApp.*")

COMMIT = "b8f4d313c41dbdef1f3404b52d83e9a71673c581"
RAW_BASE = f"https://raw.githubusercontent.com/stevenkbennett/synthetic_accessibility_project/{COMMIT}"

FILES = {
    "filip.csv": f"{RAW_BASE}/data/chemist_data/filip.csv",
    "opinions_becky.json": f"{RAW_BASE}/data/chemist_data/opinions_becky.json",
    "opinions_mebriggs.json": f"{RAW_BASE}/data/chemist_data/opinions_mebriggs.json",
}

# Recorded from this script's own first successful download of the pinned
# commit's files -- fails loudly (see --allow-hash-mismatch) if the
# upstream repo's raw content ever changes at this commit, which
# shouldn't happen for a tagged release but is checked anyway.
EXPECTED_SHA256 = {
    "filip.csv": "4f8d10f95704d9320fab58921370447ded5eaede257bd30454a3fc7e9d2aa1c9",
    "opinions_becky.json": "d21a4a551045ad9493b56c247a2bb1ca20fb6dd001ba97ad342a0aa3853a96a1",
    "opinions_mebriggs.json": "2ccb881ad2224f9b55faa776cbccdb0d659b997789b6c1e62d1a360a99cde111",
}

# Canonical SMILES of the 3 molecules found (round 22 part 3) to overlap
# exactly with TS1+TS2+TS3 -- excluded by default so this development set
# never contaminates the confirmatory benchmark.
EXACT_TS_OVERLAP = {
    "NCc1cccc2cc3cccc(CN)c3nc12",
    "N[C@@H]1CCCC[C@@H]1N",
    "Fc1ccnc(F)c1",
}


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def download_all(dest_dir: Path, allow_hash_mismatch: bool) -> dict:
    dest_dir.mkdir(parents=True, exist_ok=True)
    paths = {}
    for name, url in FILES.items():
        dest = dest_dir / name
        subprocess.run(["curl", "-sL", "--fail", "-o", str(dest), url], check=True)
        digest = sha256_of(dest)
        expected = EXPECTED_SHA256[name]
        if digest != expected:
            msg = f"{name}: sha256 {digest} != expected {expected} -- pinned commit's content changed"
            if allow_hash_mismatch:
                print(f"WARNING: {msg}", file=sys.stderr)
            else:
                raise SystemExit(f"{msg}\n(pass --allow-hash-mismatch to proceed anyway)")
        paths[name] = dest
    return paths


def canon_smiles(smi: str):
    mol = Chem.MolFromSmiles(smi)
    return Chem.MolToSmiles(mol) if mol is not None else None


def canon_inchi(inchi: str):
    mol = Chem.MolFromInchi(inchi)
    return Chem.MolToSmiles(mol) if mol is not None else None


def load_filip(path: Path) -> dict:
    out = {}
    n_fail = 0
    with open(path) as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 2:
                continue
            label, smi = parts[0], parts[1]
            c = canon_smiles(smi)
            if c is None:
                n_fail += 1
                continue
            out[c] = int(label)
    print(f"filip.csv: {len(out)} parsed, {n_fail} unparseable", file=sys.stderr)
    return out


def load_inchi_json(path: Path, name: str) -> dict:
    raw = json.load(open(path))
    out = {}
    n_fail = 0
    for inchi, label in raw.items():
        c = canon_inchi(inchi)
        if c is None:
            n_fail += 1
            continue
        out[c] = int(label)
    print(f"{name}: {len(out)} parsed, {n_fail} unparseable", file=sys.stderr)
    return out


def build_consensus(chemists: dict) -> list:
    """chemists: {chemist_name: {canonical_smiles: 0_or_1 (1=easy, source
    convention)}}. Returns one record per molecule in the union, with
    per-chemist votes preserved (source 1=easy convention, unflipped),
    plus n_raters/agreement_fraction/hard_consensus (flipped to
    this project's hard=1 convention; None if raters split exactly 1-1
    with no majority).
    """
    names = list(chemists.keys())
    union = set()
    for votes in chemists.values():
        union |= set(votes)

    records = []
    for smi in sorted(union):
        votes = {name: chemists[name].get(smi) for name in names}
        present = [v for v in votes.values() if v is not None]
        n_raters = len(present)
        vote_counts = Counter(present)
        majority_label, majority_count = vote_counts.most_common(1)[0]
        agreement_fraction = majority_count / n_raters
        is_tie = n_raters == 2 and vote_counts[0] == vote_counts[1]
        consensus_easy = None if is_tie else majority_label
        hard_consensus = None if consensus_easy is None else (1 - consensus_easy)
        records.append(
            {
                "smiles": smi,
                "n_raters": n_raters,
                "agreement_fraction": agreement_fraction,
                "hard_consensus": hard_consensus,
                **{f"chemist_{name}": votes[name] for name in names},
            }
        )
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    parser.add_argument("--allow-hash-mismatch", action="store_true")
    parser.add_argument(
        "--include-ts-overlap",
        action="store_true",
        help="do not exclude the 3 molecules that exactly overlap TS1/TS2/TS3 (default: excluded)",
    )
    args = parser.parse_args()

    raw_paths = download_all(args.output_dir / "raw", args.allow_hash_mismatch)

    chemists = {
        "filip": load_filip(raw_paths["filip.csv"]),
        "becky": load_inchi_json(raw_paths["opinions_becky.json"], "opinions_becky.json"),
        "mebriggs": load_inchi_json(raw_paths["opinions_mebriggs.json"], "opinions_mebriggs.json"),
    }

    records = build_consensus(chemists)
    n_before_exclusion = len(records)
    if not args.include_ts_overlap:
        records = [r for r in records if r["smiles"] not in EXACT_TS_OVERLAP]
    n_excluded = n_before_exclusion - len(records)

    out_dir = args.output_dir / "mpscore"
    out_dir.mkdir(parents=True, exist_ok=True)

    smi_path = out_dir / "mpscore_dev.smi"
    labels_path = out_dir / "mpscore_dev_labels.csv"
    with open(smi_path, "w") as smi_f, open(labels_path, "w", newline="") as labels_f:
        writer = csv.writer(labels_f)
        writer.writerow(["id", "hard_consensus", "n_raters", "agreement_fraction", "chemist_filip", "chemist_becky", "chemist_mebriggs"])
        for i, r in enumerate(records):
            mol_id = f"MPSCORE_{i:05d}"
            smi_f.write(f"{r['smiles']}\t{mol_id}\n")
            writer.writerow(
                [
                    mol_id,
                    "" if r["hard_consensus"] is None else r["hard_consensus"],
                    r["n_raters"],
                    f"{r['agreement_fraction']:.4f}",
                    "" if r["chemist_filip"] is None else r["chemist_filip"],
                    "" if r["chemist_becky"] is None else r["chemist_becky"],
                    "" if r["chemist_mebriggs"] is None else r["chemist_mebriggs"],
                ]
            )

    n_raters_dist = Counter(r["n_raters"] for r in records)
    n_ties = sum(1 for r in records if r["hard_consensus"] is None)
    print(f"wrote {len(records)} molecules ({n_excluded} excluded as exact TS1/2/3 overlap) to {smi_path}, {labels_path}", file=sys.stderr)
    print(f"n_raters distribution: {dict(sorted(n_raters_dist.items()))}", file=sys.stderr)
    print(f"undefined consensus (2-rater 1-1 ties, hard_consensus blank): {n_ties}", file=sys.stderr)


if __name__ == "__main__":
    main()
