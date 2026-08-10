#!/usr/bin/env python3
"""Scores a molecule set with the official BR-SAScore PyPI package
(`BRSAScore.BRSAScore.SAScorer`, default config: reaction_from='uspto',
buildingblock_from='emolecules').

Score direction (verified from source, see ../datasets/README.md): 1
(easy) to 10 (hard) -- same scale as SAscore.

Input format: whitespace/tab-delimited `<smiles> <id>` per line.

Output: JSONL, one record per input line:
`{"id": ..., "smiles": ..., "brsascore": <float|null>, "error": <str|null>}`
"""

import argparse
import json
import sys
import warnings
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    warnings.filterwarnings("ignore")  # RDKit deprecation noise from BRSAScore's own Morgan fingerprint call
    from BRSAScore.BRSAScore import SAScorer
    from rdkit import Chem, RDLogger

    RDLogger.DisableLog("rdApp.*")
    scorer = SAScorer()

    n_ok = 0
    n_err = 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.input) as fin, open(args.output, "w") as fout:
        for line in fin:
            line = line.strip()
            if not line:
                continue
            smiles, mol_id = line.split("\t") if "\t" in line else line.split(None, 1)
            if Chem.MolFromSmiles(smiles) is None:
                fout.write(json.dumps({"id": mol_id, "smiles": smiles, "brsascore": None, "error": "unparseable"}) + "\n")
                n_err += 1
                continue
            try:
                score, _contribution = scorer.calculateScore(smiles)
            except Exception as exc:  # BR-SAScore's own code raises bare exceptions on some inputs (e.g. empty fingerprints)
                fout.write(json.dumps({"id": mol_id, "smiles": smiles, "brsascore": None, "error": str(exc)}) + "\n")
                n_err += 1
                continue
            fout.write(json.dumps({"id": mol_id, "smiles": smiles, "brsascore": score, "error": None}) + "\n")
            n_ok += 1

    print(f"wrote {n_ok + n_err} records ({n_ok} ok, {n_err} errors) to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
