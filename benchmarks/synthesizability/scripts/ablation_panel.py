#!/usr/bin/env python3
"""Development-set ablation panel for diagnosing the two TS2/TS3 failure
modes found in the round-22 benchmark (see ../../docs/benchmark.md and
../DEVELOPMENT_SET.md).

This panel carries NO easy/hard ground-truth labels. It cannot be used
to compute accuracy, and must never be used that way. Its only purpose
is to localize which of yomitoki's five components responds to which
structural axis, by holding everything but one axis constant and reading
the score. This is diagnosis, not tuning -- no weight/threshold/formula
is touched by this script or by reading its output.

Every molecule is generated from a fixed SMILES literal below (the
selection rule IS this code, not a prose description of one) and
verified twice before use:
  1. against RDKit's own descriptor computation (the axis this molecule
     is meant to represent actually holds, e.g. "stereo=2" really is 2)
  2. against TS1/TS2/TS3's full molecule sets, for zero overlap (this is
     a fresh panel, not warmed-over test-set molecules)

Three axes, chosen because they isolate the two failure-mode hypotheses
from ../../docs/benchmark.md's TS2/TS3 stratification:

  FM-1 (fused aromatic ring count 1->4, stereocenters=0 throughout,
        no bridging/spiro): TS2/TS3's false positives skew toward large,
        ring-rich, highly aromatic molecules with almost no bridging/
        spiro/macrocycle complexity -- benzene -> naphthalene ->
        anthracene -> tetracene isolates "does the score respond to
        aromatic ring *fusion count* alone."

  FM-2 (stereocenter count 0->4, ring count=1 and heavy-atom count
        roughly constant throughout): TS2/TS3's false negatives skew
        toward small, few-ring molecules that nonetheless carry
        stereocenters -- a cyclohexane with an increasingly
        halogen-substituted chain isolates "does the score respond to
        stereocenter count in a small/simple-ring molecule."

  CTRL (bridgehead atom count 0/2/4, a control, not a failure-mode
        target): cyclohexane -> norbornane -> adamantane. This is
        genuine 3D structural complexity, the kind ring_topology is
        actually meant to detect. If the score does NOT respond here
        either, the defect is "the components are blind," a different
        and worse finding than "the components are miscalibrated" (FM-1/
        FM-2 alone would only show the latter).
"""

import csv
import json
import subprocess
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
from rdkit import Chem, RDLogger  # noqa: E402
from rdkit.Chem import rdMolDescriptors  # noqa: E402

RDLogger.DisableLog("rdApp.*")

SCRIPT_DIR = Path(__file__).parent
DATASETS_DIR = SCRIPT_DIR.parent / "datasets" / "downloaded" / "brsascore"
RESULTS_DIR = SCRIPT_DIR.parent / "results"

# (id, smiles, expected_heavy_atoms, expected_rings, expected_stereocenters, expected_bridgeheads, expected_spiro)
FM1_SERIES = [
    ("FM1_R1_benzene", "c1ccccc1", 6, 1, 0, 0, 0),
    ("FM1_R2_naphthalene", "c1ccc2ccccc2c1", 10, 2, 0, 0, 0),
    ("FM1_R3_anthracene", "c1ccc2cc3ccccc3cc2c1", 14, 3, 0, 0, 0),
    ("FM1_R4_tetracene", "c1ccc2cc3cc4ccccc4cc3cc2c1", 18, 4, 0, 0, 0),
]

FM2_SERIES = [
    ("FM2_S0_none", "C1CCCCC1CCCC", 10, 1, 0, 0, 0),
    ("FM2_S1_one", "C1CCCCC1C(F)CCC", 11, 1, 1, 0, 0),
    ("FM2_S2_two", "C1CCCCC1C(F)C(Cl)CC", 12, 1, 2, 0, 0),
    ("FM2_S3_three", "C1CCCCC1C(F)C(Cl)C(Br)C", 13, 1, 3, 0, 0),
    ("FM2_S4_four", "C1CCCCC1C(F)C(Cl)C(Br)C(I)C", 15, 1, 4, 0, 0),
]

CTRL_SERIES = [
    ("CTRL_B0_cyclohexane", "C1CCCCC1", 6, 1, 0, 0, 0),
    ("CTRL_B2_norbornane", "C1CC2CCC1C2", 7, 2, None, 2, 0),
    ("CTRL_B4_adamantane", "C1C2CC3CC1CC(C2)C3", 10, 4, None, 4, 0),
]

ALL_SERIES = {"FM1": FM1_SERIES, "FM2": FM2_SERIES, "CTRL": CTRL_SERIES}


def verify_descriptors(entries):
    """Asserts each molecule's RDKit descriptors match what the axis
    claims to hold constant/vary -- fails loudly, not silently, if a
    hand-written SMILES doesn't actually represent its intended point on
    the axis. `None` in the expected-stereocenters slot skips that one
    check (used for the CTRL series, where RDKit's unassigned-stereocenter
    detector flags bridgehead-adjacent atoms as potential stereocenters
    even though the whole molecule is achiral by symmetry -- a known
    RDKit quirk unrelated to what CTRL is testing, so not asserted on).
    """
    for mol_id, smi, exp_heavy, exp_rings, exp_stereo, exp_bridge, exp_spiro in entries:
        mol = Chem.MolFromSmiles(smi)
        assert mol is not None, f"{mol_id}: unparseable SMILES {smi!r}"
        heavy = mol.GetNumHeavyAtoms()
        rings = mol.GetRingInfo().NumRings()
        stereo = len(Chem.FindMolChiralCenters(mol, includeUnassigned=True, useLegacyImplementation=False))
        bridge = rdMolDescriptors.CalcNumBridgeheadAtoms(mol)
        spiro = rdMolDescriptors.CalcNumSpiroAtoms(mol)
        assert heavy == exp_heavy, f"{mol_id}: heavy_atoms {heavy} != expected {exp_heavy}"
        assert rings == exp_rings, f"{mol_id}: rings {rings} != expected {exp_rings}"
        if exp_stereo is not None:
            assert stereo == exp_stereo, f"{mol_id}: stereocenters {stereo} != expected {exp_stereo}"
        assert bridge == exp_bridge, f"{mol_id}: bridgeheads {bridge} != expected {exp_bridge}"
        assert spiro == exp_spiro, f"{mol_id}: spiro {spiro} != expected {exp_spiro}"


def canonical(smi):
    mol = Chem.MolFromSmiles(smi)
    return Chem.MolToSmiles(mol) if mol is not None else None


def verify_no_overlap_with_benchmark_sets(entries):
    """Zero overlap with TS1/TS2/TS3 by construction -- verified, not
    assumed. Compares canonical SMILES, not raw strings or ids, so a
    differently-written SMILES for the same molecule would still be
    caught.
    """
    panel_canon = {canonical(smi) for _, smi, *_ in entries}
    benchmark_canon = set()
    for ds in ["ts1", "ts2", "ts3"]:
        path = DATASETS_DIR / f"{ds}.smi"
        if not path.exists():
            print(f"WARNING: {path} not found, cannot verify overlap against {ds}", file=sys.stderr)
            continue
        with open(path) as f:
            for line in f:
                smi = line.split("\t")[0].strip()
                c = canonical(smi)
                if c:
                    benchmark_canon.add(c)
    overlap = panel_canon & benchmark_canon
    if overlap:
        raise SystemExit(f"REFUSING TO PROCEED: {len(overlap)} panel molecule(s) also appear in TS1/TS2/TS3: {overlap}")
    print(f"verified: 0/{len(panel_canon)} panel molecules overlap with TS1+TS2+TS3 ({len(benchmark_canon)} unique canonical SMILES)", file=sys.stderr)


def write_panel_smi(entries, out_path):
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w") as f:
        for mol_id, smi, *_ in entries:
            f.write(f"{smi}\t{mol_id}\n")


def run_yomitoki(input_path, output_path):
    bin_path = SCRIPT_DIR.parent.parent.parent / "target" / "release" / "yomitoki"
    if not bin_path.exists():
        raise SystemExit(f"yomitoki binary not found at {bin_path} -- build with: cargo build --release --bin yomitoki")
    subprocess.run(
        [sys.executable, str(SCRIPT_DIR / "run_yomitoki.py"), str(input_path), str(output_path), "--bin", str(bin_path)],
        check=True,
    )


def summarize(output_path):
    records = {}
    with open(output_path) as f:
        for line in f:
            r = json.loads(line)
            records[r["id"]] = r

    print("\n=== Ablation panel: overall.difficulty and component contributions ===\n")
    for series_name, entries in ALL_SERIES.items():
        print(f"--- {series_name} ---")
        header = f"{'id':28s} {'difficulty':>10s} {'ring':>8s} {'size':>8s} {'stereo':>8s} {'fg':>8s}"
        print(header)
        for mol_id, *_ in entries:
            r = records.get(mol_id)
            if r is None or r["error"] is not None:
                print(f"{mol_id:28s} ERROR: {r['error'] if r else 'missing'}")
                continue
            c = r["yomitoki_components"]
            print(
                f"{mol_id:28s} {r['yomitoki_difficulty']:10.4f} "
                f"{c['ring_topology']['contribution']:8.4f} "
                f"{c['size_topology']['contribution']:8.4f} "
                f"{c['stereochemical_burden']['contribution']:8.4f} "
                f"{c['functional_group_liability']['contribution']:8.4f}"
            )
        print()


def main():
    all_entries = [e for series in ALL_SERIES.values() for e in series]
    verify_descriptors(all_entries)
    verify_no_overlap_with_benchmark_sets(all_entries)

    panel_smi = RESULTS_DIR / "ablation_panel.smi"
    panel_out = RESULTS_DIR / "ablation_panel.jsonl"
    write_panel_smi(all_entries, panel_smi)
    run_yomitoki(panel_smi, panel_out)
    summarize(panel_out)


if __name__ == "__main__":
    main()
