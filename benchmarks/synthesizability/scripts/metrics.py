"""Statistics for the synthesizability benchmark: standard binary
classification metrics (via scikit-learn, not reimplemented), bootstrap
confidence intervals, selective-prediction risk-coverage/AURC, and
confidence-vs-correctness calibration.

Every function here takes already-normalized inputs: `y_true` is 0/1
(1 = the "hard"/positive class per the benchmark's own label
convention -- see ../README.md), `y_score` is a continuous score where
higher always means "more likely hard" (direction normalization for each
method happens in normalize_direction.py, not here -- this module never
guesses a score's meaning).

No dataset-specific or competitor-specific code lives here on purpose --
this module is exercised directly by the CI smoke test
(test_metrics.py) against small synthetic fixtures, independent of
whether any real benchmark data is available in the CI environment.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field

import numpy as np
from sklearn.metrics import (
    average_precision_score,
    balanced_accuracy_score,
    confusion_matrix,
    f1_score,
    matthews_corrcoef,
    precision_score,
    recall_score,
    roc_auc_score,
)


@dataclass
class ClassificationMetrics:
    n: int
    roc_auc: float
    pr_auc: float
    accuracy: float
    balanced_accuracy: float
    precision: float
    recall: float
    f1: float
    mcc: float
    tn: int
    fp: int
    fn: int
    tp: int

    def to_dict(self) -> dict:
        return {
            "n": self.n,
            "roc_auc": self.roc_auc,
            "pr_auc": self.pr_auc,
            "accuracy": self.accuracy,
            "balanced_accuracy": self.balanced_accuracy,
            "precision": self.precision,
            "recall": self.recall,
            "f1": self.f1,
            "mcc": self.mcc,
            "confusion_matrix": {"tn": self.tn, "fp": self.fp, "fn": self.fn, "tp": self.tp},
        }


def classification_metrics(y_true, y_score, threshold: float = 0.5) -> ClassificationMetrics:
    """`y_score` must already be in `0.0..=1.0` (or any consistent scale)
    with higher = more likely positive. `threshold` binarizes for the
    metrics that need a hard prediction (accuracy/precision/recall/F1/MCC/
    confusion matrix); AUC metrics are threshold-free by construction.
    """
    y_true = np.asarray(y_true, dtype=int)
    y_score = np.asarray(y_score, dtype=float)
    if len(np.unique(y_true)) < 2:
        raise ValueError(
            "classification_metrics requires both classes present in y_true "
            f"(got unique values: {np.unique(y_true)}) -- AUC is undefined otherwise"
        )
    y_pred = (y_score >= threshold).astype(int)

    tn, fp, fn, tp = confusion_matrix(y_true, y_pred, labels=[0, 1]).ravel()
    return ClassificationMetrics(
        n=len(y_true),
        roc_auc=roc_auc_score(y_true, y_score),
        pr_auc=average_precision_score(y_true, y_score),
        accuracy=(tp + tn) / len(y_true),
        balanced_accuracy=balanced_accuracy_score(y_true, y_pred),
        precision=precision_score(y_true, y_pred, zero_division=0),
        recall=recall_score(y_true, y_pred, zero_division=0),
        f1=f1_score(y_true, y_pred, zero_division=0),
        mcc=matthews_corrcoef(y_true, y_pred),
        tn=int(tn),
        fp=int(fp),
        fn=int(fn),
        tp=int(tp),
    )


def bootstrap_ci(
    y_true,
    y_score,
    metric_fn=lambda yt, ys: roc_auc_score(yt, ys),
    n_bootstrap: int = 1000,
    ci: float = 0.95,
    rng_seed: int = 0,
) -> dict:
    """Percentile bootstrap CI for any metric_fn(y_true, y_score) -> float.
    Resamples molecule indices with replacement, so class balance and
    correlated errors within a resample are preserved -- not a naive
    per-class bootstrap.

    `rng_seed` is a required, explicit argument (never `Date.now()`-style
    implicit randomness) so a reported CI is exactly reproducible from the
    same inputs.
    """
    y_true = np.asarray(y_true, dtype=int)
    y_score = np.asarray(y_score, dtype=float)
    n = len(y_true)
    rng = np.random.default_rng(rng_seed)
    point = metric_fn(y_true, y_score)

    values = []
    for _ in range(n_bootstrap):
        idx = rng.integers(0, n, size=n)
        yt, ys = y_true[idx], y_score[idx]
        if len(np.unique(yt)) < 2:
            continue  # degenerate resample (all one class) -- skip, don't crash
        values.append(metric_fn(yt, ys))

    if not values:
        return {"point": point, "low": float("nan"), "high": float("nan"), "n_valid_resamples": 0}

    alpha = (1.0 - ci) / 2.0
    low = float(np.quantile(values, alpha))
    high = float(np.quantile(values, 1.0 - alpha))
    return {"point": float(point), "low": low, "high": high, "n_valid_resamples": len(values)}


def paired_bootstrap_diff_ci(
    y_true,
    y_score_before,
    y_score_after,
    metric_fn=lambda yt, ys: roc_auc_score(yt, ys),
    n_bootstrap: int = 1000,
    ci: float = 0.95,
    rng_seed: int = 0,
) -> dict:
    """CI for `metric_fn(after) - metric_fn(before)` on the *same*
    molecules (e.g. a before/after model-change comparison), using the
    same resampled indices for both scores each round -- not two
    independent `bootstrap_ci` calls. Independent CIs can each be wide
    and overlapping while the paired difference is still tight and
    significant (or vice versa); pairing is what makes the CI actually
    answer "did this change help on this population," not "are these
    two numbers merely each plausible on their own."
    """
    y_true = np.asarray(y_true, dtype=int)
    y_score_before = np.asarray(y_score_before, dtype=float)
    y_score_after = np.asarray(y_score_after, dtype=float)
    n = len(y_true)
    rng = np.random.default_rng(rng_seed)
    point = metric_fn(y_true, y_score_after) - metric_fn(y_true, y_score_before)

    diffs = []
    for _ in range(n_bootstrap):
        idx = rng.integers(0, n, size=n)
        yt = y_true[idx]
        if len(np.unique(yt)) < 2:
            continue
        diffs.append(metric_fn(yt, y_score_after[idx]) - metric_fn(yt, y_score_before[idx]))

    if not diffs:
        return {"point": point, "low": float("nan"), "high": float("nan"), "n_valid_resamples": 0}

    alpha = (1.0 - ci) / 2.0
    low = float(np.quantile(diffs, alpha))
    high = float(np.quantile(diffs, 1.0 - alpha))
    return {"point": float(point), "low": low, "high": high, "n_valid_resamples": len(diffs)}


@dataclass
class RiskCoveragePoint:
    coverage: float
    threshold: float
    n_covered: int
    risk: float  # error rate among covered predictions
    balanced_error: float
    precision: float
    recall: float


def risk_coverage_curve(y_correct, confidence, coverage_levels=None) -> list[RiskCoveragePoint]:
    """Selective prediction: sort by confidence descending, and at each
    requested coverage level, report the error rate ("risk") among the
    most-confident `coverage` fraction of predictions.

    `y_correct`: 1 if the prediction was correct, 0 if wrong -- already a
    per-molecule correctness label, not a class label (this function
    doesn't know or care what the underlying classification task is).
    `confidence`: the model's own confidence for that prediction (yomitoki's
    `overall.confidence`, or 1.0 uniformly for methods with no confidence
    concept -- the caller decides).
    """
    if coverage_levels is None:
        coverage_levels = [1.0, 0.99, 0.95, 0.90, 0.80, 0.70]
    y_correct = np.asarray(y_correct, dtype=int)
    confidence = np.asarray(confidence, dtype=float)
    n = len(y_correct)
    order = np.argsort(-confidence, kind="stable")
    sorted_correct = y_correct[order]
    sorted_confidence = confidence[order]

    points = []
    for cov in coverage_levels:
        k = max(1, int(round(cov * n)))
        covered = sorted_correct[:k]
        errors = 1 - covered
        risk = float(errors.mean())
        # Balanced error rate among covered: average of the two classes'
        # error rates, treating "correct"/"incorrect" as the two classes,
        # since risk-coverage only tracks correctness, not the original
        # positive/negative label -- this is *not* the same balanced
        # accuracy as classification_metrics above, kept separate on
        # purpose so the two never get conflated in a report.
        precision = float(covered.mean())  # fraction of covered that are correct
        recall = float(covered.sum() / max(1, y_correct.sum()))
        points.append(
            RiskCoveragePoint(
                coverage=k / n,
                threshold=float(sorted_confidence[k - 1]),
                n_covered=k,
                risk=risk,
                balanced_error=1.0 - precision,  # same as risk here; kept as a named field for report clarity
                precision=precision,
                recall=recall,
            )
        )
    return points


def aurc(y_correct, confidence) -> float:
    """Area Under the Risk-Coverage curve, via trapezoidal integration over
    every distinct coverage level the sorted-by-confidence ordering
    actually produces (not just the coarse reporting levels above) --
    the standard definition (Geifman & El-Yaniv 2017): lower is better,
    a perfect confidence ranking that defers all errors to low coverage
    gives a small AURC.
    """
    y_correct = np.asarray(y_correct, dtype=int)
    confidence = np.asarray(confidence, dtype=float)
    n = len(y_correct)
    order = np.argsort(-confidence, kind="stable")
    sorted_correct = y_correct[order]
    cumulative_errors = np.cumsum(1 - sorted_correct)
    coverages = np.arange(1, n + 1) / n
    risks = cumulative_errors / np.arange(1, n + 1)
    # Manual trapezoidal integration -- np.trapz was removed in numpy 2.0
    # in favor of np.trapezoid, which doesn't exist before numpy 2.0, so
    # neither name is safe to call across the numpy versions this project
    # might reasonably pin; this one-liner has no version dependency.
    return float(np.sum((risks[1:] + risks[:-1]) / 2.0 * np.diff(coverages)))


@dataclass
class CalibrationBin:
    bin_low: float
    bin_high: float
    n: int
    mean_confidence: float
    observed_accuracy: float
    error_rate: float


def calibration_bins(y_correct, confidence, bin_edges=None) -> list[CalibrationBin]:
    if bin_edges is None:
        bin_edges = [0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0001]  # 1.0001 so confidence==1.0 falls in the last bin
    y_correct = np.asarray(y_correct, dtype=int)
    confidence = np.asarray(confidence, dtype=float)
    bins = []
    for lo, hi in zip(bin_edges[:-1], bin_edges[1:]):
        mask = (confidence >= lo) & (confidence < hi)
        n = int(mask.sum())
        if n == 0:
            bins.append(CalibrationBin(lo, hi, 0, float("nan"), float("nan"), float("nan")))
            continue
        acc = float(y_correct[mask].mean())
        bins.append(
            CalibrationBin(
                bin_low=lo,
                bin_high=min(hi, 1.0),
                n=n,
                mean_confidence=float(confidence[mask].mean()),
                observed_accuracy=acc,
                error_rate=1.0 - acc,
            )
        )
    return bins


def brier_score(y_correct, confidence) -> float:
    """Mean squared error between confidence and observed correctness.
    Only a meaningful "calibration" metric if confidence is intended as a
    correctness-probability estimate -- report alongside an explicit note
    when the method under test (e.g. yomitoki) doesn't claim that
    semantics; still valid as a ranking-usefulness signal either way.
    """
    y_correct = np.asarray(y_correct, dtype=float)
    confidence = np.asarray(confidence, dtype=float)
    return float(np.mean((confidence - y_correct) ** 2))


def expected_calibration_error(bins: list[CalibrationBin], n_total: int) -> float:
    ece = 0.0
    for b in bins:
        if b.n == 0:
            continue
        ece += (b.n / n_total) * abs(b.observed_accuracy - b.mean_confidence)
    return ece
