#!/usr/bin/env python3
"""Section 10: controlled composition-series panel -- semantic sanity
only, not used to select a candidate. Same-approximate-L1,
different-composition series: does a candidate separate molecules L1
collapses together, without pathological jumps or overpenalizing common
simple compounds?

Baseline component contributions (ring/stereo/fg) come from one real
CLI run against the current 0.2.0-alpha.1 binary; per-candidate scores
reuse the verified closed-form aggregation + fixed saturation transform
rather than rebuilding Rust per candidate.
"""

import json
import subprocess
import warnings

from rdkit import Chem, RDLogger

from common import (
    AGGREGATORS,
    RESULTS_DIR,
    SIZE_WEIGHT_PER_HETEROATOM,
    SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT,
    SIZE_WEIGHT_PER_ROTATABLE_BOND,
    overall_difficulty,
    saturate,
)

warnings.filterwarnings("ignore")
RDLogger.DisableLog("rdApp.*")

from pathlib import Path  # noqa: E402

YOMITOKI_BIN = Path(__file__).resolve().parents[3] / "target" / "release" / "yomitoki"

DIFFICULTY_LIKELY_ACCESSIBLE_MAX = 0.25
DIFFICULTY_MODERATE_MAX = 0.5
DIFFICULTY_CHALLENGING_MAX = 0.75


def verdict(d):
    if d <= DIFFICULTY_LIKELY_ACCESSIBLE_MAX:
        return "LikelyAccessible"
    if d <= DIFFICULTY_MODERATE_MAX:
        return "ModeratelyAccessible"
    if d <= DIFFICULTY_CHALLENGING_MAX:
        return "Challenging"
    return "HighlyChallenging"


# (a) same MW/heavy-atom count, different flexibility: verified via RDKit
# before use -- 2,2,4,4-tetramethylpentane and 3,3-diethylpentane are
# exact MW/heavy-atom isomers (both MW 128.26, 9 heavy atoms, 0 rings,
# 0 heteroatoms), differing only in rotatable-bond count (0 vs 4) --
# isolates flexibility composition with MW and ring/fg/stereo held fixed.
MW_VS_FLEX_SERIES = [
    ("high_mw_zero_flex", "CC(C)(C)CC(C)(C)C"),  # 2,2,4,4-tetramethylpentane: MW 128.26, RB 0
    ("same_mw_high_flex", "CCC(CC)(CC)CC"),  # 3,3-diethylpentane: MW 128.26, RB 4
]

# (b) heteroatom-dominant vs. rotatable-bond-dominant burden, roughly
# matched heavy-atom count -- verified via RDKit: low_rb_high_het has 4
# heteroatoms (het_burden 0.12) and RB 4; high_rb_low_het has 0
# heteroatoms and RB 5 (rb_burden 0.15) -- not a perfect L1 match (no
# such match exists at this heavy-atom count without confounding rings),
# but the dominant-term identity is clearly swapped, which is what this
# check needs.
HET_VS_RB_SERIES = [
    ("low_rb_high_het", "COC(OC)(OC)OC"),  # tetramethyl orthocarbonate: het=4, RB=4, MW=136.15
    ("high_rb_low_het", "CCCCCCCC"),  # octane: het=0, RB=5, MW=114.23
]

# (c) common easy heterocycles -- must not swing toward HighlyChallenging.
EASY_HETEROCYCLES = [
    ("pyridine", "c1ccncc1"),
    ("morpholine", "C1COCCN1"),
]

# (d) a common simple compound with 0 rotatable bonds, 0 heteroatoms --
# MAX aggregation's single-dominant-term behavior should not overpenalize
# this relative to L1.
SIMPLE_COMPOUNDS = [
    ("cyclohexane", "C1CCCCC1"),
    ("benzene", "c1ccccc1"),
]

# (e) a large, flexible, all-carbon molecule -- MAX should not make this
# unnaturally "easy" by ignoring MW/heteroatom burden once RB dominates.
LARGE_FLEXIBLE = [
    ("long_flexible_chain", "CCCCCCCCCCCCCCCCCCCC"),  # C20, 0 rings, 0 heteroatoms, high RB
]

ALL_ENTRIES = MW_VS_FLEX_SERIES + HET_VS_RB_SERIES + EASY_HETEROCYCLES + SIMPLE_COMPOUNDS + LARGE_FLEXIBLE


def run_cli(entries):
    input_path = RESULTS_DIR / "_panel.smi"
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(input_path, "w") as f:
        for mol_id, smi in entries:
            f.write(f"{smi}\t{mol_id}\n")
    result = subprocess.run([str(YOMITOKI_BIN), "analyze", "--input", str(input_path), "--format", "jsonl"], capture_output=True, text=True, check=True)
    input_path.unlink()
    return {json.loads(line)["input"]: json.loads(line)["report"] for line in result.stdout.splitlines()}


records = run_cli(ALL_ENTRIES)

panel_report = []
for mol_id, smi in ALL_ENTRIES:
    rep = records[mol_id]
    comps = rep["components"]
    ring_c, stereo_c, fg_c = comps["ring_topology"]["contribution"], comps["stereochemical_burden"]["contribution"], comps["functional_group_liability"]["contribution"]

    mol = Chem.MolFromSmiles(smi)
    from rdkit.Chem import Descriptors, rdMolDescriptors  # noqa: E402

    mw = Descriptors.MolWt(mol)
    rb = rdMolDescriptors.CalcNumRotatableBonds(mol)
    het = sum(1 for a in mol.GetAtoms() if a.GetSymbol() not in ("C", "H"))
    m = SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT * mw
    r = SIZE_WEIGHT_PER_ROTATABLE_BOND * rb
    h = SIZE_WEIGHT_PER_HETEROATOM * het

    by_candidate = {}
    for name, agg_fn in AGGREGATORS.items():
        raw = agg_fn(m, r, h)
        size_c = saturate(raw)
        d = float(overall_difficulty(ring_c, size_c, stereo_c, fg_c))
        by_candidate[name] = {"size_contribution": round(size_c, 4), "difficulty": round(d, 4), "verdict": verdict(d)}

    panel_report.append({"id": mol_id, "smiles": smi, "mw": mw, "rotatable_bonds": rb, "heteroatom_count": het, "mw_burden": m, "rotatable_burden": r, "heteroatom_burden": h, "by_candidate": by_candidate})

with open(RESULTS_DIR / "controlled_panel.json", "w") as f:
    json.dump(panel_report, f, indent=2)

print(f"{'id':<22} {'mw_b':>6} {'rb_b':>6} {'het_b':>6}  " + "  ".join(f"{n:>10}" for n in AGGREGATORS))
for r in panel_report:
    cells = "  ".join(f"{r['by_candidate'][n]['difficulty']:>10.4f}" for n in AGGREGATORS)
    print(f"{r['id']:<22} {r['mw_burden']:>6.3f} {r['rotatable_burden']:>6.3f} {r['heteroatom_burden']:>6.3f}  {cells}")
