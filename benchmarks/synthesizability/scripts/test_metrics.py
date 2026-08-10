"""Smoke tests for metrics.py, run against small synthetic fixtures only --
no RDKit, no downloaded dataset, no network access. This is what CI runs
(see ../../.github/workflows/ci.yml's benchmark-smoke job): it verifies the
statistics implementation itself is correct, independent of whether any
real benchmark data is available in the CI environment.
"""

import math

import numpy as np
import pytest

from metrics import (
    aurc,
    bootstrap_ci,
    brier_score,
    calibration_bins,
    classification_metrics,
    expected_calibration_error,
    risk_coverage_curve,
)


def test_classification_metrics_perfect_separation():
    y_true = [0, 0, 0, 1, 1, 1]
    y_score = [0.1, 0.2, 0.3, 0.7, 0.8, 0.9]
    m = classification_metrics(y_true, y_score)
    assert m.roc_auc == 1.0
    assert m.pr_auc == 1.0
    assert m.accuracy == 1.0
    assert m.mcc == 1.0
    assert (m.tn, m.fp, m.fn, m.tp) == (3, 0, 0, 3)


def test_classification_metrics_random_guessing_is_worse_than_perfect():
    rng = np.random.default_rng(0)
    y_true = rng.integers(0, 2, size=500)
    y_score = rng.random(500)  # uncorrelated with y_true
    m = classification_metrics(y_true, y_score)
    assert 0.35 < m.roc_auc < 0.65  # should hover near 0.5, generous bounds to avoid flakiness


def test_classification_metrics_requires_both_classes():
    with pytest.raises(ValueError):
        classification_metrics([1, 1, 1], [0.1, 0.2, 0.3])


def test_bootstrap_ci_is_reproducible_given_the_same_seed():
    y_true = [0, 1] * 50
    y_score = list(np.linspace(0, 1, 100))
    a = bootstrap_ci(y_true, y_score, n_bootstrap=200, rng_seed=42)
    b = bootstrap_ci(y_true, y_score, n_bootstrap=200, rng_seed=42)
    assert a == b


def test_bootstrap_ci_contains_the_point_estimate():
    y_true = [0, 1] * 50
    y_score = list(np.linspace(0, 1, 100))
    result = bootstrap_ci(y_true, y_score, n_bootstrap=500, rng_seed=1)
    assert result["low"] <= result["point"] <= result["high"]


def test_risk_coverage_curve_monotonic_when_confidence_tracks_correctness():
    # Perfectly-ranked confidence: the most confident predictions are all
    # correct, least confident are all wrong -- risk must never increase
    # as coverage decreases (dropping the least-confident predictions
    # first can only remove errors or leave risk unchanged).
    n = 200
    y_correct = [1] * 150 + [0] * 50
    confidence = list(np.linspace(1.0, 0.0, n))  # highest confidence first, matching y_correct's order
    points = risk_coverage_curve(y_correct, confidence, coverage_levels=[1.0, 0.9, 0.7, 0.5])
    risks = [p.risk for p in points]
    # coverage_levels is given high->low, so risk must be non-increasing:
    # dropping the least-confident predictions first can only remove
    # errors or leave risk unchanged.
    assert risks == sorted(risks, reverse=True)
    assert points[-1].risk == 0.0  # at 50% coverage, only the perfectly-correct half remains


def test_risk_coverage_curve_flat_when_confidence_is_uninformative():
    rng = np.random.default_rng(0)
    y_correct = rng.integers(0, 2, size=1000)
    confidence = rng.random(1000)  # uncorrelated with correctness
    points = risk_coverage_curve(y_correct, confidence, coverage_levels=[1.0, 0.5])
    # Risk at 100% coverage is just the overall error rate; at 50% coverage
    # with uninformative confidence it should be close to the same value,
    # not dramatically lower -- generous tolerance to avoid flakiness.
    assert abs(points[0].risk - points[1].risk) < 0.15


def test_aurc_is_lower_for_better_ranked_confidence():
    n = 200
    y_correct = np.array([1] * 150 + [0] * 50)
    good_confidence = np.linspace(1.0, 0.0, n)  # matches y_correct's order exactly
    rng = np.random.default_rng(0)
    bad_confidence = rng.random(n)  # uninformative
    good_aurc = aurc(y_correct, good_confidence)
    bad_aurc = aurc(y_correct, bad_confidence)
    assert good_aurc < bad_aurc


def test_calibration_bins_and_ece_are_zero_for_perfectly_calibrated_input():
    # Confidence exactly equals observed accuracy in every bin by
    # construction -> ECE should be ~0.
    rng = np.random.default_rng(0)
    confidence = np.array([0.55] * 1000)
    y_correct = (rng.random(1000) < 0.55).astype(int)
    bins = calibration_bins(y_correct, confidence, bin_edges=[0.5, 0.6])
    assert len(bins) == 1
    assert bins[0].n == 1000
    ece = expected_calibration_error(bins, n_total=1000)
    assert ece < 0.05  # sampling noise only


def test_brier_score_zero_for_perfect_confident_correct_predictions():
    y_correct = [1, 1, 1, 1]
    confidence = [1.0, 1.0, 1.0, 1.0]
    assert brier_score(y_correct, confidence) == 0.0


def test_brier_score_positive_for_overconfident_wrong_predictions():
    y_correct = [0, 0, 0, 0]
    confidence = [1.0, 1.0, 1.0, 1.0]
    assert brier_score(y_correct, confidence) == 1.0
