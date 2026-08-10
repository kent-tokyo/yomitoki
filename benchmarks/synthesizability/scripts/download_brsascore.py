#!/usr/bin/env python3
"""Downloads and verifies the BR-SAScore TS1/TS2/TS3 test set.

Provenance (verified directly against the downloaded file, not assumed --
see benchmarks/synthesizability/datasets/README.md for the full writeup):

  - Origin paper: Chen & Jung, "Estimating the synthetic accessibility of
    molecules with building block and reaction-aware SAScore", J.
    Cheminform. 16, 83 (2024). https://doi.org/10.1186/s13321-024-00879-0
  - Official repo (github.com/snu-micc/BR-SAScore) returns HTTP 404 as of
    this writing -- confirmed dead, not a transient error. A Wayback
    Machine snapshot from 2024-07-24 confirms it existed and listed a
    data/ directory, but no archived copy of that directory's contents
    was found.
  - This script downloads from an UNOFFICIAL third-party mirror
    (github.com/awadell1/BR-SAScore), the only currently-reachable copy
    found. Its authenticity relative to the vanished original cannot be
    cryptographically verified -- there is no official checksum to check
    against. Use with that caveat explicitly in mind.
  - TS1's class counts (745 ES / 1,055 HS) match the paper's own table
    exactly. TS2's and TS3's do not match exactly (paper: TS2 858/942,
    TS3 810/990; this file: TS2 784/351 with 665 additional rows whose
    text `accessibility` field is blank -- see below -- TS3 747/1053).
    Whether this reflects a different snapshot, minor postprocessing
    differences, or mirror alteration could not be determined.
  - Despite the `accessibility` text column being blank for 665 TS2 rows,
    the numeric `labels` column (1=hard-to-synthesize/HS, 0=easy/ES) is
    fully populated and internally consistent for all 5,400 rows
    (0 label/accessibility mismatches among the 4,735 non-blank rows) --
    `labels` is therefore used as ground truth, not the partially
    -populated `accessibility` text column.
  - Verified independently: 0 duplicate ids, 0 duplicate SMILES strings,
    1/5400 RDKit-unparseable SMILES (excluded, not silently dropped --
    see the exclusion count this script reports).

License: no explicit license is stated for the TS1/TS2/TS3 label file
itself. The underlying molecule sources (ZINC-15, GDB-17, ChEMBL) each
carry their own terms; the BR-SAScore *code* (not this data file) is
MIT-licensed per its PyPI package metadata.
"""

import argparse
import csv
import hashlib
import subprocess
import sys
from pathlib import Path

MIRROR_URL = "https://raw.githubusercontent.com/awadell1/BR-SAScore/main/data/test_set.csv"
# Recorded from this script's own first successful download -- pins the
# exact bytes this benchmark was built against, not a claim about the
# "true original" (see module docstring: no official checksum exists to
# verify against).
EXPECTED_SHA256 = "93621d56743bbe14c97bd0dc846e5f9e48f46ed7e140c57f85cf8fb48bc8e4d9"
EXPECTED_ROW_COUNT = 5400


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def download(dest: Path, allow_hash_mismatch: bool) -> Path:
    dest.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["curl", "-sL", "--fail", "-o", str(dest), MIRROR_URL], check=True)
    digest = sha256_of(dest)
    if digest != EXPECTED_SHA256:
        msg = (
            f"downloaded file sha256 {digest} != expected {EXPECTED_SHA256} "
            f"-- the unofficial mirror's content changed since this script was written"
        )
        if allow_hash_mismatch:
            print(f"WARNING: {msg}", file=sys.stderr)
        else:
            raise SystemExit(f"{msg}\n(pass --allow-hash-mismatch to proceed anyway)")
    return dest


def preprocess(raw_path: Path, out_dir: Path) -> None:
    """Splits into TS1/TS2/TS3, using `labels` as ground truth (not the
    partially-populated `accessibility` text column -- see module doc),
    excluding RDKit-unparseable SMILES with the exclusion count reported,
    never silently dropped.
    """
    from rdkit import Chem
    from rdkit import RDLogger

    RDLogger.DisableLog("rdApp.*")

    with open(raw_path) as f:
        rows = list(csv.DictReader(f))
    if len(rows) != EXPECTED_ROW_COUNT:
        print(
            f"WARNING: expected {EXPECTED_ROW_COUNT} rows, got {len(rows)} -- "
            f"the mirror's content may have changed",
            file=sys.stderr,
        )

    by_dataset: dict[str, list[dict]] = {}
    excluded_unparseable = 0
    sanitized_ids = 0
    for row in rows:
        if Chem.MolFromSmiles(row["smiles"]) is None:
            excluded_unparseable += 1
            continue
        # A real data-quality wrinkle in the mirror's own `id` column: 928
        # of TS2's 1800 ids contain an embedded space (e.g.
        # "GDB ChEMBL21844") -- whitespace-tokenizing readers (yomitoki's
        # CLI batch reader, in particular) mis-split these into two
        # fields, silently collapsing hundreds of distinct molecules onto
        # one garbage id ("GDB") and corrupting coverage. Sanitized here,
        # at the source, so every downstream consumer of the .smi/labels
        # files sees the same (fixed) id -- not patched around in one
        # consumer's parser only. Undirectional (space/tab -> underscore),
        # reported below, never silent.
        raw_id = row["id"]
        sanitized_id = "_".join(raw_id.split())
        if sanitized_id != raw_id:
            sanitized_ids += 1
        row = {**row, "id": sanitized_id}
        by_dataset.setdefault(row["dataset"], []).append(row)

    out_dir.mkdir(parents=True, exist_ok=True)
    for dataset, dataset_rows in sorted(by_dataset.items()):
        out_path = out_dir / f"{dataset.lower()}.smi"
        with open(out_path, "w") as f:
            for row in dataset_rows:
                f.write(f"{row['smiles']}\t{row['id']}\n")
        ground_truth_path = out_dir / f"{dataset.lower()}_labels.csv"
        with open(ground_truth_path, "w") as f:
            f.write("id,label\n")
            for row in dataset_rows:
                f.write(f"{row['id']},{row['labels']}\n")
        n_hard = sum(1 for r in dataset_rows if r["labels"] == "1")
        n_easy = len(dataset_rows) - n_hard
        print(
            f"{dataset}: {len(dataset_rows)} molecules ({n_hard} hard/HS, {n_easy} easy/ES) "
            f"-> {out_path}, {ground_truth_path}",
            file=sys.stderr,
        )

    print(f"excluded (RDKit-unparseable): {excluded_unparseable}", file=sys.stderr)
    print(f"ids sanitized (embedded whitespace -> underscore): {sanitized_ids}", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    parser.add_argument(
        "--allow-hash-mismatch",
        action="store_true",
        help="proceed even if the downloaded file's sha256 doesn't match the pinned value",
    )
    args = parser.parse_args()

    raw_path = args.output_dir / "raw" / "brsascore_test_set.csv"
    download(raw_path, args.allow_hash_mismatch)
    print(f"downloaded, sha256={sha256_of(raw_path)}", file=sys.stderr)
    preprocess(raw_path, args.output_dir / "brsascore")


if __name__ == "__main__":
    main()
