# Final Holdout One-Shot Results (round 22 part 20, Phase 6+)

Opened once, per `HOLDOUT_MANIFEST.md` (pre-opening commit `bac0758`).
Every endpoint, threshold, comparator, and stratum below was fixed
*before* this file existed. **These are the results as obtained — no
formula, weight, threshold, or molecule-selection rule was touched
after seeing them, per the manifest's own rules.**

## Primary table

Spearman ρ (correlation between each method's score and real
patent-literature route length) is the pre-registered **primary**
endpoint. ROC-AUC/PR-AUC/BalAcc/MCC are the pre-registered **secondary**
view, using the fixed `route_steps > 3` binarization and a
scale-invariant rank-based operating point (top-N by score, N = the
number of "hard" molecules under that binarization — see
`analyze_results.py` for why an absolute cutoff doesn't work across
YOMITOKI's 0–1 scale and SAscore/BR-SAScore's 1–10 scale).

| Method | Spearman ρ | Pearson r | ROC-AUC | PR-AUC | BalAcc | MCC | FP | FN |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| YOMITOKI v0.1.1 | 0.1585 | 0.1468 | 0.5876 | 0.2197 | 0.5292 | 0.0584 | 1419 | 1419 |
| **YOMITOKI v0.2.0-alpha.1** | **0.1491** | 0.1362 | 0.5818 | 0.2156 | 0.5206 | 0.0411 | 1445 | 1445 |
| SAscore | 0.0765 | 0.0631 | 0.5599 | 0.2059 | 0.5139 | 0.0279 | 1465 | 1465 |
| BR-SAScore | 0.0687 | 0.0494 | 0.5441 | 0.1968 | 0.5023 | 0.0046 | 1500 | 1500 |

All correlations statistically significant at N=9,996 (p < 1e-11
throughout) — **significance is not in question, magnitude is**.
ρ=0.149 is a weak monotonic relationship by any conventional standard.

## Pairwise (paired bootstrap, 1,000 resamples, seed 0, 95% CI)

| Comparison | Spearman ρ diff | 95% CI | ROC-AUC diff | 95% CI |
|---|---:|---|---:|---|
| v0.2 − v0.1.1 | **−0.0094** | [−0.0158, −0.0037] | **−0.0058** | [−0.0107, −0.0015] |
| v0.2 − SAscore | +0.0726 | [0.0537, 0.0915] | +0.0219 | [0.0086, 0.0347] |
| v0.2 − BR-SAScore | +0.0804 | [0.0623, 0.0972] | +0.0376 | [0.0241, 0.0506] |

**v0.2.0-alpha.1 is statistically significantly *worse* than v0.1.1 on
this holdout** — both CIs are entirely negative, not just centered
below zero. It remains clearly, significantly better than both external
comparators on the same metric and subset.

## Structural strata (Phase 8, v0.2.0-alpha.1, median split fixed pre-opening)

| Stratum | Low half (n) | ρ (low) | High half (n) | ρ (high) |
|---|---:|---:|---:|---:|
| Molecular weight | 4,998 | 0.119 | 4,998 | **0.023** |
| Ring count | 6,545 | 0.161 | 3,451 | **−0.001** |
| Aromatic ring count | 5,906 | 0.157 | 4,090 | 0.057 |
| Stereocenter count | 6,763 | 0.169 | 3,233 | 0.075 |
| Heteroatom count | 5,245 | 0.152 | 4,090 | 0.064 |

The already-weak overall correlation is not evenly spread — it is
**concentrated almost entirely in structurally simpler molecules** and
**collapses to essentially zero for the high-ring-count half**
(ρ=−0.001, indistinguishable from no relationship at all). YOMITOKI's
difficulty score tracks real route length only weakly among simple
molecules and not at all among ring-complex ones, on this dataset.

## TS1/TS2/TS3 regression check (Phase 9, run only after primary results were saved)

| Benchmark | v0.1.1 ROC-AUC | v0.2.0-alpha.1 ROC-AUC | Δ |
|---|---:|---:|---:|
| TS1 | 0.9543 | 0.9563 | +0.0019 |
| TS2 | 0.5075 | 0.5841 | **+0.0766** |
| TS3 | 0.6805 | 0.7463 | **+0.0658** |

TS1's strong region is preserved (not damaged, marginally improved).
TS2 and TS3 both show the real, meaningful improvement the v0.2
development work targeted and validated on MPScore. **This is not a
substitute for the primary holdout result and does not override it**
— it is reported per Phase 9's own stated purpose, a regression check
against previously-established benchmarks, nothing more.

## The honest tension, not resolved by picking a favorite number

v0.2.0-alpha.1's scoring changes (ring-family L2 aggregation,
heteroatom-count burden) generalize positively to TS1–3 — the same
population family MPScore itself broadly resembles (ZINC/GDB/ChEMBL
-sourced, screening-library-flavored drug-like molecules) — but do not
clearly generalize, and by the primary metric slightly regress, on
PaRoutes' genuinely different question: does the score track *real,
historically-executed* route length among molecules chemists already
know how to make. Both readings are real; neither cancels the other.
The pre-registered gate structure exists precisely so this tension gets
resolved by a rule fixed in advance, not by whichever number is more
flattering after the fact.

## Decision (Phase 10, applied exactly as pre-registered)

Primary endpoint ρ = **0.1491**, below the pre-registered 0.20 floor.
v0.2.0-alpha.1 is also statistically significantly worse than v0.1.1 on
this same metric (CI entirely negative). Per §8 of the manifest:

> **NO-GO**: ρ < 0.20, or worse than v0.1.1, or a serious structural
> regression.

Both NO-GO conditions independently fire (ρ<0.20, *and* worse than
v0.1.1) plus the structural-strata collapse is itself a form of the
third condition. **Verdict: NO-GO for a stable `v0.2.0` release on the
strength of this holdout.**

Per the manifest's own bug-correction policy (§9) and Phase 10's NO-GO
branch: this holdout is not reopened, not rescored, and not used to
retry a fix. No implementation bug was found in the course of this
analysis — the result stands as obtained. Any future scoring change is
`v0.3` development against a new, not-yet-opened holdout.
