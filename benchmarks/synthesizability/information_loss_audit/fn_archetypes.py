#!/usr/bin/env python3
"""Information-Loss Audit, step 5 (round 22 part 13): does the FN
population organize into a handful of describable archetypes, using the
richer F1 raw descriptors this round built?

KMeans, k=3..8, selected by silhouette score on the standardized F1
descriptor space -- restricted to FN molecules only (a fresh clustering,
distinct from chemical_space/analyze_errors.py's whole-population UMAP
clustering). If no k in that range clears a modest silhouette floor,
that itself is reported as "the data doesn't support a clean
archetype split" rather than forcing one.
"""

import json
import sys
import warnings

import numpy as np
import pandas as pd
from sklearn.cluster import KMeans
from sklearn.metrics import silhouette_score
from sklearn.preprocessing import StandardScaler

warnings.filterwarnings("ignore")

from common import F0_COLUMNS, F1_COLUMNS, RESULTS_DIR, SEED

SILHOUETTE_FLOOR = 0.10  # below this, don't claim a meaningful archetype split


def pick_k(X, k_range=range(3, 9)):
    scores = {}
    for k in k_range:
        km = KMeans(n_clusters=k, random_state=SEED, n_init=10)
        labels = km.fit_predict(X)
        if len(set(labels)) < 2:
            continue
        scores[k] = float(silhouette_score(X, labels))
    return scores


def archetype_report(df, labels, k):
    df = df.copy()
    df["archetype"] = labels
    n_total = len(df)
    reports = []
    for cid in sorted(set(labels)):
        group = df[df["archetype"] == cid]
        finding_counts = {}
        for codes in group["finding_codes"].fillna(""):
            for code in codes.split(";"):
                code = code.strip()
                if code:
                    finding_counts[code] = finding_counts.get(code, 0) + 1
        top_findings = sorted(finding_counts.items(), key=lambda kv: -kv[1])[:5]
        reports.append(
            {
                "archetype": int(cid),
                "n": int(len(group)),
                "share_of_all_fn": float(len(group) / n_total),
                "mean_descriptors": {c: float(group[c].mean()) for c in F1_COLUMNS},
                "mean_component_scores": {c: float(group[c].mean()) for c in F0_COLUMNS},
                "mean_agreement_fraction": float(group["agreement_fraction"].mean()),
                "mean_n_raters": float(group["n_raters"].mean()),
                "dominant_finding_codes": [{"code": c, "n": n} for c, n in top_findings],
            }
        )
    return reports


def label_archetype(report, global_means):
    """Short, evidence-grounded label from descriptor deviations -- same
    discipline as chemical_space/analyze_errors.py's structural_
    interpretation: describe what's actually different, don't invent a
    name the numbers don't support.
    """
    parts = []
    d = report["mean_descriptors"]
    for feat, label in [
        ("stereocenters", "stereo-rich"),
        ("ring_count", "ring-rich"),
        ("heavy_atoms", "large"),
        ("aromatic_ring_count", "aromatic-heavy"),
        ("fg_count", "FG-dense"),
        ("rotatable_bonds", "flexible"),
        ("fraction_csp3", "saturated"),
    ]:
        gm = global_means[feat]
        if gm == 0:
            continue
        ratio = d[feat] / gm
        if ratio >= 1.4:
            parts.append(f"high-{label}")
        elif ratio <= 0.6:
            parts.append(f"low-{label}")
    return "+".join(parts) if parts else "no strong deviation from global FN mean"


def main():
    df = pd.read_csv(RESULTS_DIR / "features_with_folds.csv")
    fn = df[df["confusion"] == "FN"].reset_index(drop=True)

    X = fn[F1_COLUMNS].to_numpy()
    X_std = StandardScaler().fit_transform(X)

    scores = pick_k(X_std)
    print(f"silhouette by k: {scores}", file=sys.stderr)
    best_k = max(scores, key=scores.get) if scores else None
    best_score = scores.get(best_k) if best_k else None

    result = {
        "n_fn": int(len(fn)),
        "silhouette_by_k": scores,
        "silhouette_floor": SILHOUETTE_FLOOR,
        "best_k": best_k,
        "best_silhouette": best_score,
        "data_supports_archetypes": bool(best_score is not None and best_score >= SILHOUETTE_FLOOR),
    }

    global_means = {c: float(fn[c].mean()) for c in F1_COLUMNS}
    result["global_fn_means"] = global_means

    if result["data_supports_archetypes"]:
        km = KMeans(n_clusters=best_k, random_state=SEED, n_init=10)
        labels = km.fit_predict(X_std)
        reports = archetype_report(fn, labels, best_k)
        for r in reports:
            r["label"] = label_archetype(r, global_means)
        result["archetypes"] = reports
    else:
        # Report the best-available k's clusters anyway, explicitly
        # flagged as weak -- still useful for the round's own honesty
        # norms (show what was tried, don't hide a negative result).
        if best_k is not None:
            km = KMeans(n_clusters=best_k, random_state=SEED, n_init=10)
            labels = km.fit_predict(X_std)
            reports = archetype_report(fn, labels, best_k)
            for r in reports:
                r["label"] = label_archetype(r, global_means)
            result["weak_archetypes_not_claimed_as_real"] = reports

    with open(RESULTS_DIR / "fn_archetypes.json", "w") as f:
        json.dump(result, f, indent=2)

    print(f"n_fn={result['n_fn']} best_k={best_k} best_silhouette={best_score} supports_archetypes={result['data_supports_archetypes']}", file=sys.stderr)
    if "archetypes" in result:
        for r in result["archetypes"]:
            print(f"  archetype {r['archetype']}: n={r['n']} ({r['share_of_all_fn']*100:.1f}%) label={r['label']}", file=sys.stderr)
    elif "weak_archetypes_not_claimed_as_real" in result:
        print("  (weak split, not claimed as real archetypes):", file=sys.stderr)
        for r in result["weak_archetypes_not_claimed_as_real"]:
            print(f"  archetype {r['archetype']}: n={r['n']} ({r['share_of_all_fn']*100:.1f}%) label={r['label']}", file=sys.stderr)


if __name__ == "__main__":
    main()
