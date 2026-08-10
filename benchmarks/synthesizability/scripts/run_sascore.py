#!/usr/bin/env python3
"""Scores a molecule set with RDKit's bundled SAscore reference
implementation (`rdkit.Contrib.SA_Score.sascorer`).

Score direction (verified from source, see ../datasets/README.md): 1
(easy) to 10 (hard).

Input format: whitespace/tab-delimited `<smiles> <id>` per line, matching
run_yomitoki.py and download_brsascore.py's output.

Output: JSONL, one record per input line:
`{"id": ..., "smiles": ..., "sascore": <float|null>, "error": <str|null>}`
"""

import argparse
import json
import sys
from pathlib import Path

# rdkit.Contrib isn't on sys.path by default -- it ships inside the rdkit
# package but as a plain (non-package) directory of scripts.
import rdkit

CONTRIB_SA_SCORE = Path(rdkit.__file__).parent / "Contrib" / "SA_Score"
sys.path.insert(0, str(CONTRIB_SA_SCORE))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    import sascorer  # noqa: E402 -- path must be set up first
    from rdkit import Chem, RDLogger

    RDLogger.DisableLog("rdApp.*")

    n_ok = 0
    n_err = 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.input) as fin, open(args.output, "w") as fout:
        for line in fin:
            line = line.strip()
            if not line:
                continue
            smiles, mol_id = line.split("\t") if "\t" in line else line.split(None, 1)
            mol = Chem.MolFromSmiles(smiles)
            if mol is None:
                fout.write(json.dumps({"id": mol_id, "smiles": smiles, "sascore": None, "error": "unparseable"}) + "\n")
                n_err += 1
                continue
            score = sascorer.calculateScore(mol)
            fout.write(json.dumps({"id": mol_id, "smiles": smiles, "sascore": score, "error": None}) + "\n")
            n_ok += 1

    print(f"wrote {n_ok + n_err} records ({n_ok} ok, {n_err} errors) to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
