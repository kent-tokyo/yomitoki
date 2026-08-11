# Final Size-Topology Aggregation Experiment (round 22 part 19)

Last scoring experiment before the `v0.2.0` scoring freeze. The user's
own correction to round 18's framing: the compression probe's +0.039
residual signal was scoped as a "saturation transform redesign," but
the saturation transform (`s = 1 - exp(-raw/2)`) is monotonic and
invertible (`raw = -2*ln(1-s)`) wherever it matters (`s < 1`), so a
transform-shape change can't recover information the transform itself
never destroyed. The real first-candidate hypothesis is that
**aggregating `mw_burden + rotatable_burden + heteroatom_burden` into
one scalar via a plain sum (L1) collapses composition information**,
independent of the transform. This round separates the two hypotheses
rigorously before touching anything.

**Baseline**: `0.2.0-alpha.1`. Frozen and untouched by every script in
this round: `SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT`,
`SIZE_WEIGHT_PER_ROTATABLE_BOND`, `SIZE_WEIGHT_PER_HETEROATOM` (`0.03`,
frozen round 22 part 17), `SIZE_BURDEN_SCALE` (the saturation
transform's `/2`), all four `AGGREGATE_WEIGHT_*` constants, and every
verdict threshold. MPScore development set only — no TS1/TS2/TS3, no
holdout. **Verify**: `git diff main -- src/ tests/ Cargo.toml` returns
nothing; this round never touched production code.

## 1. Math check: is the saturation transform actually invertible on real data?

Yes, confirmed on all 10,543 labeled molecules — but the first pass
found something worth investigating rather than assuming away.
`raw_true = mw_burden + rotatable_burden + heteroatom_burden` vs.
`raw_reconstructed = -2*ln(1 - size_contribution)` (from the real
`0.2.0-alpha.1` binary's own output) initially disagreed by up to
**0.090** for 91 molecules (0.86% of the set) — all clustered in
sulfur-/halogen-rich structures (disulfides, dithiocarbamates,
amidines: `O=C(O)C(=S)SSC(=S)C(=O)O`, `NC(=Nc1nc(...` etc.).

Diagnosed, not assumed: this is a **data-quality bug in the cached
`rotatable_bonds` column**, not a transform bug. `information_loss_audit`'s
`features_with_folds.csv` computed `rotatable_bonds` via RDKit's
`CalcNumRotatableBonds`, which disagrees with chematic's actual
`rotatable_bond_count()` for S-S/C(=S)/amidine-type bonds — chematic
counts up to 3 more rotatable bonds than RDKit for these patterns.
Backed out the true per-molecule count as the residual after inverting
the transform (`(raw_reconstructed - mw_burden - heteroatom_burden) /
0.03`, rounded to the nearest integer — cleanly integer-valued to within
0.004 for all but a small residual explained by the already-documented
~0.22%-relative mol_wt monoisotopic/average-mass difference). After
correction: max abs diff `0.0037`, mean `0.0004`. **Verdict: the
transform is not the information-loss source.** This correction is
baked into `common.py`'s loader and used everywhere else in this round
(91 molecules affected).

## 2-3. Transform-vs-aggregation discriminator (A0-A8)

Same scaffold folds, same probe methodology as round 14/18 (L2-logistic
primary, HistGradientBoosting secondary).

| Set | Linear ROC-AUC | Nonlinear ROC-AUC |
|---|---|---|
| A0: F0 only | 0.6198 | 0.7611 |
| A1: F0 + raw total (still summed, just un-saturated) | 0.6560 (+0.036) | 0.7611 (**+0.000**) |
| A2: F0 + mw_burden alone | 0.6372 (+0.018) | 0.7638 (+0.003) |
| A3: F0 + rotatable_burden alone | 0.6780 (+0.058) | 0.7925 (+0.031) |
| A4: F0 + heteroatom_burden alone | 0.7144 (+0.095) | 0.7877 (+0.027) |
| A5: F0 + mw + rb | 0.7359 | 0.7979 |
| A6: F0 + mw + het | 0.7175 | 0.8009 |
| A7: F0 + rb + het | 0.7231 | 0.8073 |
| A8: F0 + mw + rb + het, decomposed | **0.7423 (+0.122)** | **0.8106 (+0.050)** |

**A1's nonlinear delta is exactly 0.000 — not approximately zero,
exactly.** This is mathematically guaranteed, not empirical: a
tree-based model is invariant to any monotonic transform of a feature
it already has, and `raw_total_reconstructed` is a monotonic
transform of `size_contribution` (already in F0). So the nonlinear
probe's A1≈A0 result is closer to a control/sanity check than new
evidence — it just re-confirms section 1's finding by construction.
A1's linear gain (+0.036) is a real but much smaller effect, and is
itself an artifact of linear models being sensitive to nonlinear
reshapings of an existing input (the same class of effect diagnosed in
round 18's collinearity finding) — not evidence of missing information.

**A8 vs. A1 is the clean test, and it's decisive**: decomposing the raw
total into three *separate* features adds dramatically more than simply
having access to the (still-summed) raw total — linear +0.086 beyond
A1, nonlinear +0.050 beyond A1's contribution of exactly nothing.
Individually, `rotatable_burden` and `heteroatom_burden` each carry
real independent signal (nonlinear +0.031 / +0.027 alone); `mw_burden`
alone carries almost none (+0.003) — MW is already well-represented in
F0, RB and heteroatom identity are not, once summed away.

**Diagnosis: Case G (aggregation loss), not Case T (transform loss).**
The saturation transform is confirmed invertible and contributes zero
verified information loss; collapsing three distinguishable primitives
into one L1-summed scalar is what discards real, recoverable signal.
This matches the user's stated first hypothesis.

## 4. Collision analysis

Near-collision search (`|Δsize_contribution| < 0.003`, composition
distance `≥ 0.06`) found the search space too permissive to use its raw
pair count as evidence at this dataset size (1.5M pairs, 335K with
differing labels — expected given how weak `size_topology` alone is;
not a meaningful statistic on its own). The **top-ranked examples by
composition distance are the real evidence**, and they're stark:

- `MPSCORE_02393` (mw_burden=0.428, rb_burden=0.450, het_burden=0.18,
  **label=hard, predicted=easy**) vs. `MPSCORE_06498` (mw_burden=0.759,
  rb_burden=0.000, het_burden=0.30, **label=easy, predicted=hard**) —
  composition distance 0.90, `size_contribution` differs by **0.00018**.
  Nearly opposite compositions, nearly identical size score, opposite
  labels, opposite (wrong, for both) predictions.
- `MPSCORE_06967` (het_burden=0.48, rb_burden=0.000, label=hard,
  correctly predicted hard) collides with at least 4 different
  RB-dominant, low-heteroatom molecules (all label=easy, all correctly
  predicted easy) at composition distances 0.72-0.79 — a
  heteroatom-heavy, RB-free molecule and several RB-heavy,
  heteroatom-light molecules land in the same narrow size band despite
  the dataset's own labels treating them oppositely.

Top 50 examples saved in `results/collision_analysis.json`. Direct,
inspectable confirmation of the statistical finding above: L1 summation
genuinely erases distinguishing composition information for real
MPScore molecules, not just in aggregate statistics.

## 5-7. Parameter-free aggregation candidates

`L1_current = m+r+h` (production today), `L2 = sqrt(m²+r²+h²)` (same
functional form already adopted for `ring_topology`'s aggregation,
round 22 part 11), `MAX = max(m,r,h)`. Saturation transform unchanged
(`1 - exp(-aggregated/2)`) for all three.

**A fourth candidate (LogSumExp) was considered and rejected before
testing**: `LogSumExp(x,0,0) = ln(exp(x)+2) ≠ x`, so it inflates the
burden of *every* molecule even with two burden terms at exactly zero —
a direct regression against invariants this project already tests for
(`small_molecule_has_low_size_burden`, zero-rotatable-bond/
zero-heteroatom molecules). L1/L2/MAX all correctly reduce to the
single active term's own value when only one burden term is nonzero
(verified in the controlled panel below); LogSumExp does not. Not worth
the complexity for a candidate that fails a basic sanity check before
any data is even involved.

Real production formula, full MPScore, paired bootstrap 95% CI vs. L1:

| Candidate | ROC-AUC | hard PR-AUC | Bal Acc | MCC | FP | FN | ΔAUC vs L1 (95% CI) |
|---|---|---|---|---|---|---|---|
| L1 (current) | 0.5706 | 0.9012 | 0.5604 | 0.1072 | 98 | 7404 | — |
| L2 | 0.5725 | 0.9031 | 0.5612 | 0.1136 | 70 | 7575 | +0.0019 [0.0008, 0.0030] |
| MAX | 0.5721 | 0.9034 | 0.5590 | 0.1111 | 68 | 7628 | +0.0015 [-0.0001, 0.0031] |

Per-fold ROC-AUC is consistent in direction across all 5 scaffold folds
for both candidates (no single-fold-driven artifact) — but the
*magnitude* is small and MAX's CI touches zero.

## 8. Residual-information test — the round's stated most important adoption metric

For each candidate: rebuild F0 with that candidate's own
`size_contribution`, then re-run the A8-style probe (candidate-F0 + raw
mw/rb/heteroatom burden as three separate extra features) to see how
much of the decomposition gap remains.

| Candidate | Residual gap (nonlinear) | Reduction vs. L1 | Meets bar (≤0.02 or ≥40% reduction)? |
|---|---|---|---|
| L1 (current) | 0.0495 | — | — |
| L2 | 0.0485 | **1.9%** | **No** |
| MAX | 0.0477 | **3.5%** | **No** |

Neither candidate comes close. A closed-form, parameter-free
recombination of the three burden terms into a single scalar recovers
almost none of the decomposition-detectable signal — consistent with
what the aggregation-comparison production numbers already showed
(ΔAUC an order of magnitude below the adoption gate). This isn't a
close call to argue about: **the mechanism diagnosis (Case G) is
correct, but the prescribed class of fix (aggregate 3 terms into 1
scalar differently) is structurally unable to solve it.** Any fixed
`f(m,r,h) → one number` still ultimately feeds one scalar through one
fixed `0.4` weight into `overall.difficulty` — it cannot reproduce what
a model with three independently-weighted inputs can do, no matter which
closed form `f` takes. Recovering the gap for real would require
`size_topology` to stop compressing to one scalar before the outer
aggregation, or the outer aggregation itself to change — both
explicitly out of scope this round (frozen `AGGREGATE_WEIGHT_*`,
one-axis-at-a-time discipline).

## 9. Chemical interpretation

Not written up as a production rationale — there is no adopted
candidate to justify. For the record, not as a post-hoc justification
for a decision already made on the numbers above: L2's zero-term
identity property and its moderate compromise between summation and
dominance did look like the more chemically defensible of the two
candidates in the controlled panel (§10) — MAX's pure-dominance
property let a single large burden term outscore two co-occurring
moderate ones, the opposite of "several distinct concerns compound."
That distinction didn't end up mattering because neither passed the
information-recovery bar.

## 10. Controlled panel (semantic sanity only)

Verified via RDKit before use, not assumed: `2,2,4,4-tetramethylpentane`
and `3,3-diethylpentane` are exact MW/heavy-atom isomers (both MW
128.26, 9 heavy atoms, 0 rings, 0 heteroatoms), differing only in
rotatable-bond count (0 vs. 4) — isolates flexibility composition with
MW and ring/fg/stereo held fixed.

| id | mw_b | rb_b | het_b | L1 | L2 | MAX |
|---|---|---|---|---|---|---|
| high_mw_zero_flex | 0.077 | 0.000 | 0.000 | 0.0151 | 0.0151 | 0.0151 |
| same_mw_high_flex | 0.077 | 0.120 | 0.000 | 0.0375 | 0.0275 | 0.0233 |
| low_rb_high_het (tetramethyl orthocarbonate) | 0.082 | 0.120 | 0.120 | 0.0594 | 0.0359 | **0.0233** |
| high_rb_low_het (octane) | 0.069 | 0.150 | 0.000 | 0.0414 | 0.0317 | **0.0289** |
| pyridine | 0.047 | 0.000 | 0.030 | 0.1104 | 0.1062 | 0.1045 |
| morpholine | 0.052 | 0.000 | 0.060 | 0.1478 | 0.1415 | 0.1377 |
| cyclohexane | 0.050 | 0.000 | 0.000 | 0.1051 | 0.1051 | 0.1051 |
| benzene | 0.047 | 0.000 | 0.000 | 0.1044 | 0.1044 | 0.1044 |
| long_flexible_chain (C20) | 0.170 | 0.510 | 0.000 | 0.1152 | 0.0943 | 0.0900 |

Findings: (1) single-nonzero-term identity confirmed for all three
forms (`high_mw_zero_flex` row identical across L1/L2/MAX) — none of
the three candidates create a pathological jump for the common case of
a molecule with only one active burden source. (2) monotonicity holds
throughout, no jumps. (3) common simple compounds (cyclohexane, benzene,
pyridine, morpholine) stay low and near-identical across candidates —
no overpenalty. (4) the large flexible chain doesn't become unnaturally
easy under L2/MAX (still clearly above the simple-compound baseline).
(5) **a real, inspectable weakness in MAX, not from data mining but from
the panel itself**: `low_rb_high_het` (two co-occurring moderate burden
sources, het=rb=0.12 each) scores *lower* than `high_rb_low_het` (one
larger source, rb=0.15 alone) under MAX (0.0233 vs. 0.0289) — MAX
inverts the "two concerns compound" ordering that L1 and L2 both
preserve, because it only ever looks at the single largest term. This
was a real finding from the panel, kept even though it doesn't change
the final verdict (both candidates already failed §8 regardless).

## 11-13. Real binary implementation, archetypes, regression

**Not performed.** Per this round's own gate (§14) and stop rule (§15):
no candidate cleared the residual-information bar, so there is no
selected candidate to implement, evaluate on the real binary, or check
for archetype/regression effects against. Implementing either L2 or
MAX in Rust anyway, after §8 already found <4% gap reduction, would be
producing a change whose own diagnostic evidence says it doesn't fix
the identified problem — exactly the "tiny improvement, reject" case
§14 pre-registered.

## 14. Adoption gate

| Criterion | L2 | MAX |
|---|---|---|
| Mechanism diagnosis is Case G or clear Case T | Case G, yes (shared) | Case G, yes (shared) |
| Candidate directly fixes that mechanism | **No — §8 says no** | **No — §8 says no** |
| Real-binary ΔAUC ≥ +0.015 | No (+0.0019) | No (+0.0015, CI touches 0) |
| Scaffold-fold-consistent improvement | Yes (small, consistent) | Yes (small, consistent) |
| Residual raw-information gap clearly shrinks | **No (1.9%)** | **No (3.5%)** |
| BalAcc/MCC not worsened | Improved slightly | Improved slightly |
| FP/FN tradeoff acceptable | FP↓28, FN↑171 | FP↓30, FN↑224 |
| Multiple major FN archetypes affected | Not tested (§12 skipped) | Not tested (§12 skipped) |
| FM-1/heteroatom improvements preserved | Not tested (§13 skipped) | Not tested (§13 skipped) |
| Parameter-free / minimal | Yes | Yes |
| Clear chemical semantic contract | Plausible (§9) | Weaker (§9's MAX finding) |

Both candidates fail the round's two hardest, most decision-relevant
bars (§8's residual-information reduction, §14's ΔAUC≥0.015) by a wide
margin — not a borderline judgment call. No candidate reaches
implementation.

## 15. Stop rule: REJECT

Per the round's explicit instruction: since no aggregation candidate
sufficiently improves matters and the residual information gap does not
meaningfully shrink, **no new scoring candidate is searched for**.
**`0.2.0-alpha.1`'s scoring is frozen as of this round.** MPScore-driven
scoring search ends here, regardless of outcome, per the round's own
framing (§16) — this was always the last one, win or lose.

## 16-17. What this round leaves for the record

The mechanism diagnosis itself (Case G, confirmed via a tautology-free
nonlinear-probe test and direct collision evidence) is a genuine,
durable finding even though the prescribed fix didn't work: `size_topology`
compressing three distinguishable primitives into one L1-summed scalar
before a single fixed downstream weight is applied is a real, identified
information bottleneck. Closing it for real would need a bigger
architectural change (reporting the three burdens with independently
learned/weighted downstream treatment, or restructuring
`overall.difficulty`'s own aggregation) than "swap the norm" — out of
scope for a frozen-baseline, parameter-light, one-axis-at-a-time round,
and out of scope for the `v0.2.0` freeze generally. Worth recording as
a scoped, named candidate for whichever future phase revisits
`size_topology`'s architecture, not as unfinished business for this
phase.

Also recorded, not to be re-litigated lightly: the `rotatable_bonds`
data-quality bug found in §1 (91 molecules, RDKit/chematic definition
mismatch for S-S/C(=S)/amidine bonds) exists in
`information_loss_audit/results/features_with_folds.csv`, which every
round since round 13 has reused verbatim. It did not change any prior
round's conclusion (checked: round 15/16/17's real-binary evaluations
never depended on this cached column, only this round's own offline
reconstruction did) but is worth knowing about if that CSV is ever
reused again.

Branch: `experiment/size-topology-aggregation-redesign`. `main`
untouched. Not pushed, not merged — commit only, per instruction.
