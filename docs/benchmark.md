# External synthesizability benchmark (v0.1.0 frozen baseline)

This document reports yomitoki v0.1.0's accuracy against external ground
truth, compared against SAscore and BR-SAScore, plus a selective-prediction
(confidence/abstention) evaluation. It is the first external benchmark run
against yomitoki since the crate's design has centered on explainability,
API design, and reproducibility rather than accuracy claims — this
document exists to make the accuracy question measurable instead of
assumed.

**Everything below reflects yomitoki v0.1.0's frozen, untuned default
configuration (`AnalysisConfig::default()`, `GeneralOrganic`, no fragment
corpus).** No weight, threshold, cap, or formula was changed while
reviewing these results, and none of the numbers below were used to
retune anything — see "Test-set integrity" below.

Full methodology, scripts, and machine-readable results:
`benchmarks/synthesizability/`.

**Reading TS2 and the later PaRoutes/semantic-ceiling results below**:
yomitoki reports **intrinsic structural synthesizability** only (v0.3
product decision, round 22 part 23 —
`benchmarks/synthesizability/v03_two_axis_product_framing/README.md`).
TS2's chance-level result (§ Accuracy) and the PaRoutes final holdout's
weak, high-ring-collapsing route-length correlation (`benchmarks/
synthesizability/final_holdout/RESULTS.md`) are exactly the evidence
that motivated that decision — both measure something closer to
route-dependent difficulty (does a route exist within N steps; how many
steps did the real route take), an axis this project now explicitly
holds is out of yomitoki's structural scope by design, not a shortfall
to keep chasing with further target-only tuning.

## Dataset

**TS1 / TS2 / TS3**, from Chen & Jung, "Estimating the synthetic
accessibility of molecules with building block and reaction-aware
SAScore" (BR-SAScore), *J. Cheminform.* 16, 83 (2024),
[doi:10.1186/s13321-024-00879-0](https://doi.org/10.1186/s13321-024-00879-0).
Label definition: a molecule is "ES" (easy, label 0) if a Retro*
retrosynthesis planner found a route of ≤10 steps from purchasable
building blocks, "HS" (hard, label 1) otherwise. TS1/TS2/TS3 draw their
molecule pools from three earlier papers (SYBA, Thakkar et al., GASA
respectively) and were independently re-labeled by BR-SAScore's authors.

| dataset | n | hard (label=1) | easy (label=0) | source population |
|---|---|---|---|---|
| TS1 | 1800 | 1055 | 745 | GDB-17 (enumerated) vs. ZINC15 (purchasable) |
| TS2 | 1800 | 900 | 900 | ChEMBL-derived (Thakkar et al.) |
| TS3 | 1799 (1 excluded, unparseable) | 1052 | 747 | GASA-derived (ChEMBL/GDB mix) |

**Provenance caveats, disclosed not hidden**: the official BR-SAScore
repository (`github.com/snu-micc/BR-SAScore`) is dead (404); this data is
from an unofficial mirror (`github.com/awadell1/BR-SAScore`) whose
authenticity relative to the vanished original cannot be cryptographically
verified. This project's own class counts for TS1 match the paper's
published table exactly; TS2 and TS3 do not (TS2 comes out exactly
balanced 900/900 here vs. the paper's 858/942; TS3 is 747/1052 here vs.
810/990). One data-quality bug was found and fixed at the source during
this round: 928 of TS2's 1800 `id` values contained an embedded space
(e.g. `"GDB ChEMBL21844"`), which corrupted whitespace-based parsing —
sanitized (space → underscore) in `download_brsascore.py`, applied
identically to every method's input so no method is advantaged or
disadvantaged by the fix. Full detail: `benchmarks/synthesizability/datasets/README.md`.

**No leakage analysis was performed against yomitoki's own fragment
corpora** (ChEMBL/ORD/SynRXN) for this round — `fragment_precedent` does
not feed `overall.difficulty` since round 21 (see below), so corpus
leakage cannot affect the headline numbers regardless.

## Methods

| method | version / commit | config | score direction |
|---|---|---|---|
| yomitoki | v0.1.0 (this repo, frozen) | `AnalysisConfig::default()`, no fragment corpus | 0 (easy) – 1 (hard), `overall.difficulty` |
| SAscore | RDKit 2025.9.3, `Contrib.SA_Score.sascorer` | default (`fpscores.pkl.gz`) | 1 (easy) – 10 (hard) |
| BR-SAScore | PyPI `BRSAScore` 0.1.1 | default (`reaction_from='uspto', buildingblock_from='emolecules'`) | 1 (easy) – 10 (hard) |
| SYBA | — | — | **not reproducible, excluded** (see below) |

Every method's raw direction was verified from source code and an
empirical spot-check (aspirin/caffeine/dodecane/pyridine), not assumed
from a paper abstract or README example — see
`benchmarks/synthesizability/scripts/normalize_direction.py` and
`benchmarks/synthesizability/datasets/README.md` for the full
verification trail. All three usable methods already report "higher =
harder," so no direction flip was needed; only the normalization
*module* is required to exist and be checked, not that it does anything
in this particular case.

**SYBA excluded, not silently substituted**: SYBA's official repo serves
its pretrained score file as a Git LFS pointer, not real data, and the
PyPI package named `syba`/`SyBA` is an entirely unrelated bacterial-gene
database tool (namespace collision, not the synthesizability method).
Full evidence: `benchmarks/synthesizability/datasets/README.md`.

Binarization thresholds (accuracy/precision/recall/F1/MCC/confusion
matrix only — ROC-AUC/PR-AUC are threshold-free and are the **primary**
metrics per this benchmark's methodology):

- **yomitoki: 0.5** — the pre-existing `DIFFICULTY_MODERATE_MAX` boundary
  in `src/rules.rs`, not fit to this data.
- **SAscore / BR-SAScore: 5.5** — the untuned midpoint of the methods'
  own stated 1–10 range. Neither method's paper publishes a recommended
  binary threshold. This convention is disclosed as imperfect below.

## Accuracy

95% CI via 1000-resample percentile bootstrap over molecule indices.

| dataset | method | coverage | ROC-AUC (95% CI) | PR-AUC (95% CI) | Accuracy | Balanced Acc. | Precision | Recall | F1 | MCC |
|---|---|---|---|---|---|---|---|---|---|---|
| TS1 | **yomitoki** | 1800/1800 | 0.952 [0.941, 0.963] | 0.975 [0.969, 0.980] | 0.918 | 0.928 | 0.987 | 0.872 | 0.926 | 0.844 |
| TS1 | SAscore | 1800/1800 | 0.981 [0.975, 0.986] | 0.989 [0.985, 0.992] | 0.733 | 0.772 | 1.000 | 0.544 | 0.705 | 0.575 |
| TS1 | BR-SAScore | 1800/1800 | 0.983 [0.978, 0.987] | 0.989 [0.986, 0.992] | 0.926 | 0.920 | 0.924 | 0.952 | 0.937 | 0.846 |
| TS2 | **yomitoki** | 1800/1800 | 0.476 [0.450, 0.503] | 0.496 [0.465, 0.529] | 0.500 | 0.500 | 0.500 | 0.368 | 0.424 | 0.000 |
| TS2 | SAscore | 1800/1800 | 0.915 [0.901, 0.927] | 0.925 [0.911, 0.936] | 0.661 | 0.661 | 0.987 | 0.327 | 0.491 | 0.434 |
| TS2 | BR-SAScore | 1800/1800 | 0.917 [0.904, 0.929] | 0.917 [0.899, 0.932] | 0.842 | 0.842 | 0.842 | 0.841 | 0.842 | 0.683 |
| TS3 | **yomitoki** | 1799/1799 | 0.673 [0.648, 0.698] | 0.746 [0.722, 0.773] | 0.625 | 0.607 | 0.669 | 0.711 | 0.689 | 0.218 |
| TS3 | SAscore | 1799/1799 | 0.839 [0.819, 0.856] | 0.875 [0.856, 0.892] | 0.463 | 0.541 | 1.000 | 0.082 | 0.151 | 0.189 |
| TS3 | BR-SAScore | 1799/1799 | 0.905 [0.890, 0.918] | 0.925 [0.911, 0.938] | 0.812 | 0.820 | 0.891 | 0.772 | 0.827 | 0.630 |

All three methods reach full coverage on all three sets — every molecule
that parsed under RDKit also produced a yomitoki report (yomitoki's own
applicability gating did not reject anything in these sets; see
Limitations).

**Reading this honestly:**

- **TS1: yomitoki is genuinely competitive.** ROC-AUC 0.952 is close to
  BR-SAScore's 0.983, and yomitoki's balanced accuracy (0.928) and MCC
  (0.844) at its own natural threshold actually **exceed** SAscore's
  (0.772 / 0.575) and roughly match BR-SAScore's (0.920 / 0.846). TS1
  pits GDB-17 enumerated cage/ring structures against ZINC15 purchasable
  building blocks — exactly the kind of structural contrast (ring
  fusion, size, stereocenters) yomitoki's four difficulty components are
  built to detect.
- **TS2: yomitoki has no discriminative power** (ROC-AUC 0.476, MCC
  0.000 — indistinguishable from chance). Diagnosed, not just observed:
  the per-class `overall.difficulty` distributions are nearly identical
  (label=0 mean 0.444 σ=0.143; label=1 mean 0.433 σ=0.151, both spanning
  the same 0.03–0.94 range). TS2's molecules are ChEMBL-derived drug-likes
  on both sides of the label boundary — ring/size/stereo/functional-group
  liability, the only inputs to `overall.difficulty`, do not separate
  "found a ≤10-step retrosynthesis route" from "did not" within a
  population that's structurally homogeneous by that measure. This is a
  genuine, chemically-explainable finding, not a bug: yomitoki's
  structural-heuristic score is not a proxy for retrosynthesis-route
  existence among molecules that are already similarly drug-like.
- **TS3: yomitoki is weaker than both competitors** (0.673 vs. 0.839 /
  0.905) but not at chance. Root-cause analysis is out of scope for this
  round (see Test-set integrity below).
- **SAscore's accuracy/precision/recall/F1/MCC numbers are an artifact of
  the untuned 5.5 threshold**, not a fair reading of SAscore's ranking
  quality — its ROC-AUC/PR-AUC (0.84–0.98) are strong throughout, but at
  threshold 5.5 it predicts almost nothing as "hard" (precision 1.000,
  recall as low as 0.082 on TS3), because SAscore rarely scores common
  organic molecules above ~5 in practice. This is exactly why ROC-AUC/PR-AUC,
  not the threshold-dependent metrics, are this benchmark's primary
  comparison.

## Selective prediction (yomitoki `overall.confidence`)

**Headline result: yomitoki's confidence signal does not function as a
usable selective-prediction mechanism on this benchmark — on TS1 it is
actively inverted.** This directly contradicts the differentiation this
benchmark set out to measure, and is reported in full rather than
omitted.

| dataset | AURC (lower=better) | risk @ 100% cov. | risk @ 90% cov. | risk @ 70% cov. |
|---|---|---|---|---|
| TS1 | 0.131 | 0.082 | 0.090 | 0.115 |
| TS2 | 0.531 | 0.500 | 0.498 | 0.482 |
| TS3 | 0.417 | 0.375 | 0.384 | 0.403 |

A working selective-prediction signal should show risk *decreasing* as
coverage decreases (dropping the least-confident predictions first
removes errors). **TS1 shows the opposite**: risk rises from 8.2% at
100% coverage to 11.5% at 70% coverage. TS3 shows the same inverted
pattern between 100% and 80% coverage. TS2 is flat (consistent with its
already-chance-level accuracy — there is nothing for confidence to
select for).

**Root cause, confirmed by cross-tabulation, not guessed**: on TS1,
`overall.confidence` takes only 3 distinct values (1.0, 0.85, 0.6), all
three driven entirely by stereo-related applicability flags —
`stereo_complete=false` gives 0.85, the rarer `stereo_uncheckable=true`
gives 0.6 (checked directly: every one of the 3/111/73 molecules at
confidence 0.6 across TS1/TS2/TS3 has exactly `stereo_uncheckable=true`
and no other applicability flag set — `disconnected`, `unusual_valence`,
and `domain_distance` never fire on any of these three datasets, so the
OutOfDomain axis specifically is not exercised here at all, only the
stereo-completeness axis is). On TS1 this correlates near-perfectly with
dataset provenance: 898/900 GDB-17 molecules get confidence 0.85 (their
canonical SMILES omit explicit stereo tags), while 897/900 ZINC15
molecules get confidence 1.0 (complete stereo). But yomitoki is *more*
accurate on GDB-17 (99.7% correct) than on ZINC15 (84.0% correct) — the
ring/size-heavy `overall.difficulty` score is well-suited to flagging
GDB-17's fused cage structures as hard, and less discriminating within
ZINC15's generally-simpler population. The result: confidence is a proxy
for "which source dataset is this molecule from," not for "is this
prediction likely correct," and on TS1 those two things point in
opposite directions.

One measurement caveat on the exact per-coverage-level numbers above
(the direction of the finding does not depend on it): because confidence
only takes 3 discrete values, `risk_coverage_curve`'s stable sort by
confidence descending resolves ties by input-file order, not by any
finer-grained confidence distinction — e.g. the "90% coverage" cut lands
inside the large tied block at confidence 1.0, so which specific
confidence-1.0 molecules are "in" vs. "out" at that exact threshold is
partly an artifact of file order. The three coarse confidence *bins*
(1.0 / 0.85 / 0.6) and their accuracy gap are not affected by this — only
the precise risk value at intermediate coverage levels within a tied
block should be read as approximate.

Confidence calibration (higher confidence should mean higher observed
accuracy — it does not, on TS1 or TS3):

| dataset | confidence bin | n | mean confidence | observed accuracy |
|---|---|---|---|---|
| TS1 | [0.6, 0.7) | 3 | 0.600 | 1.000 |
| TS1 | [0.8, 0.9) | 898 | 0.850 | **0.997** |
| TS1 | [0.9, 1.0] | 899 | 1.000 | **0.840** |
| TS2 | [0.6, 0.7) | 111 | 0.600 | 0.252 |
| TS2 | [0.8, 0.9) | 842 | 0.850 | 0.496 |
| TS2 | [0.9, 1.0] | 847 | 1.000 | 0.536 |
| TS3 | [0.6, 0.7) | 73 | 0.600 | 0.589 |
| TS3 | [0.8, 0.9) | 288 | 0.850 | **0.802** |
| TS3 | [0.9, 1.0] | 1438 | 1.000 | **0.591** |

Expected Calibration Error: TS1 0.154, TS2 0.405, TS3 0.335 — all high,
and on TS1/TS3 the miscalibration runs backwards (higher confidence, lower
accuracy), not merely noisy.

**What this means, stated plainly**: yomitoki does not claim
`overall.confidence` is a correctness-probability estimate (see
`docs/architecture.md`) — it is derived from input/applicability
completeness (parseability, stereo completeness, valence sanity), not
from any signal about whether the structural difficulty estimate itself
is likely right. This benchmark confirms that distinction empirically:
input-quality confidence and prediction-correctness are different axes,
and on this dataset they are not merely uncorrelated but anti-correlated
via a dataset-provenance confound. The "abstention as a differentiator"
argument this benchmark was designed to test **is not supported by
these results** as currently implemented. Whether `Verdict::Indeterminate` /
`OutOfDomain` (evaluated separately from confidence, and not exercised
at all in this run — see Limitations) would behave differently is not
yet measured.

## Performance

Same machine (see `results/timing.json` for full specs: `arm64`, macOS
26.5.2, `rustc 1.97.0`, release build), same TS1/TS2/TS3 inputs, one
discarded warm-up call per method before timing.

**Methodology is not identical across methods, disclosed rather than
presented as a fair apples-to-apples comparison**: yomitoki is timed as
a subprocess (`yomitoki analyze`, process startup + SMILES parsing +
full analysis combined — the CLI has no separate parse-only mode).
SAscore/BR-SAScore are timed in-process with RDKit parsing done *before*
the timed region (analyze-only). A Rust-level micro-benchmark isolating
yomitoki's analyze-only cost is listed as future work, not attempted
this round.

| method | TS1 (ms/mol) | TS2 (ms/mol) | TS3 (ms/mol) |
|---|---|---|---|
| yomitoki (subprocess, parse+analyze) | 1.9 | 4.7 | 8.5 |
| SAscore (in-process, analyze-only) | 0.20 | 0.23 | 0.32 |
| BR-SAScore (in-process, analyze-only) | 0.31 | 0.42 | 0.68 |

yomitoki is slower per-molecule than the two competitors under this
measurement, which is expected given it includes process startup and
full parsing where the competitors' timed region does not. All three
methods are fast in absolute terms (worst case 8.5ms/molecule, ~118
molecules/sec). A real, reproducible pattern independent of measurement
method: **all three methods slow down from TS1 → TS2 → TS3** (yomitoki
1.9→4.7→8.5 ms/mol, a ~4.5x spread; SAscore and BR-SAScore show the
same ordering at a smaller ~1.6-2x spread), consistent with TS3's
ChEMBL/GASA-derived molecules being structurally more complex on average
than TS1's GDB-17/ZINC15 mix. An initial, much larger apparent slowdown
on TS3 (>2 minutes for 1799 molecules, later re-measured cleanly at 15
seconds) was traced to CPU contention from other processes this project
ran concurrently during the same investigation, not a genuine yomitoki
performance issue — noted here so the number isn't mistaken for a fixed
finding if seen elsewhere in this project's history.

## Limitations

- **TS2/TS3 label sets don't exactly match the BR-SAScore paper's own
  published tables** (disclosed above and in
  `benchmarks/synthesizability/datasets/README.md`) — comparisons to
  the paper's own reported numbers (not attempted in this document)
  would not be apples-to-apples.
- **`Verdict::Indeterminate` / `OutOfDomain` were not exercised** by
  TS1/TS2/TS3 at all — every molecule in all three sets parsed cleanly
  and produced a determinate difficulty verdict. The applicability/OOD
  stratification this benchmark's methodology calls for (§7) has nothing
  to stratify on these particular datasets; a dataset containing
  genuinely malformed/exotic/out-of-domain input would be needed to
  evaluate that axis, and none of TS1/TS2/TS3 is that dataset.
- **SYBA is excluded** (not reproducible from any currently-reachable
  source — see Methods above).
- **No root-cause fix was attempted for TS2 or TS3** this round. Per
  this project's test-set-integrity rule (below), TS1/TS2/TS3 are
  confirmatory-only; any future accuracy work must be designed on a
  separate development set and TS1/TS2/TS3 re-run only afterward, with
  results explicitly labeled post-hoc.
- **Binarization thresholds for SAscore/BR-SAScore (5.5) are an untuned
  convention, not derived from either paper** — their binarized metrics
  should be read with that caveat; ROC-AUC/PR-AUC are unaffected and are
  the primary comparison.
- **Throughput methodology is not uniform across methods** (see
  Performance above) — treat the numbers as order-of-magnitude, not a
  precise ratio.
- **No leakage check was run between TS1/TS2/TS3 and yomitoki's own
  ChEMBL/ORD/SynRXN fragment corpora** — moot for `overall.difficulty`
  (fragment_precedent doesn't feed it, since round 21), but relevant if
  `fragment_precedent`'s own explanatory signal is ever benchmarked
  separately in the future.

## Test-set integrity

TS1/TS2/TS3 results have now been seen. Per this project's own
methodology (agreed before this round began), **TS1/TS2/TS3 are
confirmatory only from this point forward** — no weight, threshold, or
formula change may be designed by looking at these numbers. If future
work addresses TS2's chance-level result or TS3's gap, it must be
designed against a separate development set never used to compute the
numbers in this document, and any subsequent re-evaluation on
TS1/TS2/TS3 must be explicitly labeled a post-hoc confirmatory result,
not folded into a revised version of this table.

## Reproduction

```
cd benchmarks/synthesizability
pip install -r requirements.txt
pip install BRSAScore --no-deps   # see competitors/README.md

python3 scripts/download_brsascore.py datasets/downloaded
cargo build --release --bin yomitoki   # from repo root

for ds in ts1 ts2 ts3; do
  python3 scripts/run_yomitoki.py  datasets/downloaded/brsascore/$ds.smi results/raw/${ds}_yomitoki.jsonl
  python3 scripts/run_sascore.py   datasets/downloaded/brsascore/$ds.smi results/raw/${ds}_sascore.jsonl
  python3 scripts/run_brsascore.py datasets/downloaded/brsascore/$ds.smi results/raw/${ds}_brsascore.jsonl
done

python3 scripts/merge_and_evaluate.py
python3 scripts/benchmark_throughput.py
```

Outputs: `results/benchmark_summary.json` (accuracy + selective
prediction), `results/per_molecule.jsonl` (per-molecule, gitignored —
regenerate locally), `results/selective_prediction.csv`,
`results/timing.json`. `results/raw/` and `datasets/downloaded/` are
gitignored (regenerated, not committed — see `.gitignore` and
`benchmarks/synthesizability/datasets/README.md` for why).
