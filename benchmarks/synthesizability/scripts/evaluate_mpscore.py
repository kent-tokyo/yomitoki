#!/usr/bin/env python3
"""Evaluates the frozen v0.1.0 yomitoki baseline against the MPScore
expert-chemist development set (see download_mpscore.py and
../datasets/README.md's MPScore section).

This is a DEVELOPMENT set, not the confirmatory benchmark -- TS1/TS2/TS3
stay untouched. Headline metrics are stratified by n_raters because 86%
of MPScore's molecules have exactly one rater (see the provenance doc);
a single combined number would mostly measure agreement with one
chemist (becky), not with a panel.

No weight/threshold/formula is read or changed by this script.
"""

import argparse
import csv
import json
import sys
import warnings
from collections import Counter, defaultdict
from pathlib import Path

warnings.filterwarnings("ignore")
from rdkit import Chem, RDLogger  # noqa: E402
from rdkit.Chem import rdMolDescriptors  # noqa: E402

RDLogger.DisableLog("rdApp.*")

sys.path.insert(0, str(Path(__file__).parent))
from metrics import bootstrap_ci, classification_metrics  # noqa: E402
from sklearn.metrics import average_precision_score, roc_auc_score  # noqa: E402

SCRIPT_DIR = Path(__file__).parent
DATASETS_DIR = SCRIPT_DIR.parent / "datasets" / "downloaded" / "mpscore"
RESULTS_DIR = SCRIPT_DIR.parent / "results"
THRESHOLD = 0.5  # yomitoki's own DIFFICULTY_MODERATE_MAX -- see docs/benchmark.md's threshold rationale, unchanged here


def load_labels():
    rows = list(csv.DictReader(open(DATASETS_DIR / "mpscore_dev_labels.csv")))
    return {r["id"]: r for r in rows}


def load_smiles():
    out = {}
    with open(DATASETS_DIR / "mpscore_dev.smi") as f:
        for line in f:
            smi, mol_id = line.rstrip("\n").split("\t")
            out[mol_id] = smi
    return out


def load_yomitoki():
    out = {}
    with open(RESULTS_DIR / "raw" / "mpscore_yomitoki.jsonl") as f:
        for line in f:
            r = json.loads(line)
            if r["error"] is None:
                out[r["id"]] = r
    return out


def descriptors(smi):
    m = Chem.MolFromSmiles(smi)
    if m is None:
        return None
    ri = m.GetRingInfo()
    n_fused = sum(1 for a in m.GetAtoms() if sum(1 for ring in ri.AtomRings() if a.GetIdx() in ring) >= 2)
    return {
        "heavy_atoms": m.GetNumHeavyAtoms(),
        "rings": ri.NumRings(),
        "aromatic_rings": rdMolDescriptors.CalcNumAromaticRings(m),
        "stereocenters": len(Chem.FindMolChiralCenters(m, includeUnassigned=True, useLegacyImplementation=False)),
        "bridgeheads": rdMolDescriptors.CalcNumBridgeheadAtoms(m),
        "spiro": rdMolDescriptors.CalcNumSpiroAtoms(m),
        "fused_ring_atoms": n_fused,
    }


def accuracy_report(ids, labels, yomitoki, label_key="hard_consensus"):
    y_true, y_score = [], []
    for mol_id in ids:
        row = labels[mol_id]
        if row[label_key] == "":
            continue
        y_rec = yomitoki.get(mol_id)
        if y_rec is None:
            continue
        y_true.append(int(row[label_key]))
        y_score.append(y_rec["yomitoki_difficulty"])
    if len(set(y_true)) < 2:
        return {"n": len(y_true), "note": "insufficient class diversity"}
    cm = classification_metrics(y_true, y_score, threshold=THRESHOLD)
    roc_ci = bootstrap_ci(y_true, y_score, metric_fn=lambda yt, ys: roc_auc_score(yt, ys))
    pr_ci = bootstrap_ci(y_true, y_score, metric_fn=lambda yt, ys: average_precision_score(yt, ys))
    return {"n": len(y_true), "metrics": cm.to_dict(), "roc_auc_95ci": roc_ci, "pr_auc_95ci": pr_ci}


def component_distributions(ids, labels, yomitoki):
    by_label = defaultdict(lambda: defaultdict(list))
    for mol_id in ids:
        row = labels[mol_id]
        if row["hard_consensus"] == "":
            continue
        y_rec = yomitoki.get(mol_id)
        if y_rec is None:
            continue
        label = int(row["hard_consensus"])
        for comp, data in y_rec["yomitoki_components"].items():
            by_label[label][comp].append(data["contribution"])

    out = {}
    for label in (0, 1):
        out[label] = {}
        for comp, values in by_label[label].items():
            n = len(values)
            mean = sum(values) / n if n else float("nan")
            out[label][comp] = {"n": n, "mean_contribution": mean}
    return out


def finding_rates(ids, labels, yomitoki):
    by_label = defaultdict(Counter)
    n_by_label = Counter()
    for mol_id in ids:
        row = labels[mol_id]
        if row["hard_consensus"] == "":
            continue
        y_rec = yomitoki.get(mol_id)
        if y_rec is None:
            continue
        label = int(row["hard_consensus"])
        n_by_label[label] += 1
        for code in set(y_rec["yomitoki_findings"]):
            by_label[label][code] += 1
    return {label: {code: (c, n_by_label[label]) for code, c in counts.most_common()} for label, counts in by_label.items()}


def fm_stratification(ids, labels, smiles, yomitoki):
    """FM-1/FM-2/FM-3 checks: confusion-matrix category descriptor
    profile, plus representation counts so FM-2/FM-3 can be marked
    INCONCLUSIVE if this dataset doesn't actually contain molecules on
    that axis.
    """
    cat_desc = defaultdict(list)
    n_with_stereocenters = 0
    n_with_bridging = 0
    n_total = 0
    for mol_id in ids:
        row = labels[mol_id]
        if row["hard_consensus"] == "":
            continue
        y_rec = yomitoki.get(mol_id)
        if y_rec is None:
            continue
        smi = smiles.get(mol_id)
        d = descriptors(smi) if smi else None
        if d is None:
            continue
        n_total += 1
        if d["stereocenters"] > 0:
            n_with_stereocenters += 1
        if d["bridgeheads"] > 0 or d["spiro"] > 0:
            n_with_bridging += 1

        gt = int(row["hard_consensus"])
        pred = 1 if y_rec["yomitoki_difficulty"] >= THRESHOLD else 0
        cat = ("T" if pred == gt else "F") + ("P" if pred == 1 else "N")
        cat_desc[cat].append(d)

    summary = {}
    for cat, descs in cat_desc.items():
        n = len(descs)
        if n == 0:
            continue
        summary[cat] = {"n": n, **{k: sum(d[k] for d in descs) / n for k in descs[0]}}

    return {
        "representation": {
            "n_total": n_total,
            "n_with_stereocenters": n_with_stereocenters,
            "pct_with_stereocenters": 100 * n_with_stereocenters / n_total if n_total else 0,
            "n_with_bridging_or_spiro": n_with_bridging,
            "pct_with_bridging_or_spiro": 100 * n_with_bridging / n_total if n_total else 0,
        },
        "by_category": summary,
    }


def confidence_vs_disagreement(ids, labels, yomitoki):
    multi_rater_ids = [i for i in ids if labels[i]["n_raters"] in ("2", "3")]
    confidences = [yomitoki[i]["yomitoki_confidence"] for i in multi_rater_ids if i in yomitoki]
    conf_dist = Counter(round(c, 2) for c in confidences)
    return {
        "n_multi_rater": len(multi_rater_ids),
        "confidence_distribution": dict(conf_dist),
        "n_unique_confidence_values": len(conf_dist),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    args = parser.parse_args()

    labels = load_labels()
    smiles = load_smiles()
    yomitoki = load_yomitoki()
    all_ids = list(labels)

    n_raters_2plus = [i for i in all_ids if labels[i]["n_raters"] in ("2", "3")]
    n_raters_3 = [i for i in all_ids if labels[i]["n_raters"] == "3"]

    result = {
        "n_total_molecules": len(all_ids),
        "n_defined_consensus": sum(1 for i in all_ids if labels[i]["hard_consensus"] != ""),
        "accuracy_full": accuracy_report(all_ids, labels, yomitoki),
        "accuracy_n_raters_2plus": accuracy_report(n_raters_2plus, labels, yomitoki),
        "accuracy_n_raters_3": accuracy_report(n_raters_3, labels, yomitoki),
        "component_distributions_full": component_distributions(all_ids, labels, yomitoki),
        "finding_rates_full": finding_rates(all_ids, labels, yomitoki),
        "fm_stratification_full": fm_stratification(all_ids, labels, smiles, yomitoki),
        "confidence_vs_disagreement": confidence_vs_disagreement(all_ids, labels, yomitoki),
    }

    out_path = RESULTS_DIR / "mpscore_evaluation.json"
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2, default=str)
    print(f"wrote {out_path}", file=sys.stderr)

    print(json.dumps(result, indent=2, default=str))


if __name__ == "__main__":
    main()
