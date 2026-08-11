# Size-Topology Information Decomposition

Round 22 part 14. **Development tooling, not a YOMITOKI production
feature.** No production Rust code touched, no scoring change, no
public API change, no new production dependency, `RULESET_VERSION` not
bumped. MPScore development set only — TS1/TS2/TS3 not read, no new
final holdout opened.

## Purpose

The Information-Loss Audit (round 22 part 13) found `size_topology`'s
narrow inputs (MW + rotatable-bond count only) are where the largest
share of F1-vs-F0 information loss concentrates. But that round's
"size_detail" feature set bundled two different things together: the
*raw* values of what `size_topology` already uses (MW, rotatable
bonds) alongside three *new* descriptors it doesn't use at all
(`fraction_csp3`, `heteroatom_count`, `tpsa`). This round separates
them:

- **H1 (existing-primitive compression)**: `size_topology`'s current
  threshold/saturation transform of its own inputs (MW, rotatable
  bonds) discards their raw continuous information.
- **H2 (missing-feature)**: `fraction_csp3`/`heteroatom_count`/`tpsa`
  carry real information `size_topology` never sees at all.
- **H3**: both.

## 0. Information-Loss Audit → main

Audited (`src/`/`tests/`/`Cargo.toml` diff against `main`: zero lines)
and integrated via fast-forward merge, `48e78b4`.

## 1-2. Same scaffold folds, reused not reassigned

`common.py` loads `information_loss_audit/results/features_with_folds.
csv` directly — the exact same table, same `fold` column, same
Bemis-Murcko `GroupKFold` assignment as round 22 part 13. Re-verified
zero scaffold spans multiple folds (2148/2148 pass) and reproduced the
identical per-fold molecule/hard/easy counts. Every comparison in this
round is paired against that round's `S0_F0_only` baseline on the
identical held-out molecules.

## 3-4. Feature decomposition (S0-S6) and conditional information

`decompose.py`. Primary probe: L2-regularized logistic regression
(same config as round 22 part 13). Secondary confirmation only: shallow
`HistGradientBoostingClassifier` (`max_depth=3`, `max_iter=100`, not
tuned) — conclusions below are drawn from the linear probe, per
explicit instruction that this round measures conditional information
content, not maximum achievable accuracy.

| Set | Added to F0 | Linear AUC | Δ vs S0 | p-value |
|---|---|---|---|---|
| S0 | (baseline) | 0.6647 | — | — |
| S1 | raw MW + raw rotatable bonds | 0.6927 | +0.0280 | 0.0514 |
| S2 | `fraction_csp3` | 0.6691 | +0.0044 | 0.4304 |
| S3 | `heteroatom_count` | 0.7172 | **+0.0525** | **0.0119** |
| S4 | `tpsa` | 0.6718 | +0.0071 | 0.1033 |
| S5 | new trio (fsp3+heteroatoms+tpsa) | 0.7897 | +0.1250 | 0.0005 |
| S6 | raw MW/RB + new trio | 0.8036 | +0.1389 | 0.0002 |

**Conditional on raw MW/RB already present** (S1+X vs. S1 alone —
does each new descriptor still add independent information once the
existing-primitive compression is already undone?):

| Addition to S1 | Δ AUC | p-value |
|---|---|---|
| `fraction_csp3` | +0.0104 | 0.2303 |
| **`heteroatom_count`** | **+0.0506** | **0.0216** |
| `tpsa` | +0.0018 | 0.3882 |

**`heteroatom_count` is the only one of the three new descriptors that
is independently significant, both alone (p=0.012) and conditional on
MW/RB already being present (p=0.022).** `fraction_csp3` and `tpsa`
are not significant either way. S1's own effect (raw MW/RB) is small
and borderline (p=0.051) — real but far from "recovers most of the
gap": it accounts for only ~17% of the full F1-vs-F0 gap the prior
round measured (+0.028 of +0.165).

## 5. Feature direction (`feature_direction.py`)

| Feature | median (hard) | median (easy) | Cohen's d | univariate AUC |
|---|---|---|---|---|
| `mol_wt` | 261.0 | 264.0 | -0.227 | 0.481 |
| `rotatable_bonds` | 2.0 | 3.0 | -0.466 | 0.382 |
| `fraction_csp3` | 0.333 | 0.143 | +0.384 | 0.625 |
| `heteroatom_count` | 4.0 | 4.0 | +0.354 | 0.601 |
| `tpsa` | 47.6 | 52.0 | -0.110 | 0.463 |

**Notable, unexpected finding**: `mol_wt` and `rotatable_bonds` — the
two descriptors `size_topology` *already* penalizes as "more = harder"
— show weak-to-*backwards* univariate direction in MPScore
(`rotatable_bonds` AUC 0.382, meaningfully below chance in the naive
"more=harder" direction). This doesn't necessarily mean the current
penalty direction is globally wrong (interactions with other
components could still make the full formula net-correct), but it is
at minimum a caution against assuming H1 (fixing MW/RB's own transform)
would help much — consistent with S1's own weak, borderline result
above.

`heteroatom_count`'s IQR shifts cleanly higher for hard (3-6) vs. easy
(2-4) despite matching medians (4 vs. 4) — real distributional
separation, not just a median artifact. **Survives conditioning on
both `mol_wt` bins (AUC 0.56-0.68 in every quartile) and
`rotatable_bonds` bins (AUC 0.61-0.67 in every quartile)** — its signal
is not a proxy for molecular size.

## 6-7. Current transform audit and collision analysis (`transform_audit.py`)

Reproduced `size_topology`'s exact formula from `src/rules.rs`
(`RULESET_VERSION` 0.11.0): `raw = 0.0006*mol_wt + 0.03*rotatable_
bonds`, `normalized = 1 - exp(-raw/2.0)`. Verified against the real
`size_contribution` column (median diff exactly 0, float noise only;
~0.9% of molecules show a larger diff traced to a rotatable-bond-count
*definitional* difference between RDKit's counter and chematic's, not
a formula error — does not affect the collision analysis, which uses
the real column directly).

**Also found and fixed a data-quality issue from the prior two
rounds**: their `mol_wt` column was computed via RDKit's
`CalcExactMolWt` (monoisotopic mass), not the standard average
molecular weight chematic's own `molecular_weight()` uses (confirmed
by reading chematic-chem's source — it sums `avg_mass()` per atom).
Verified this does not meaningfully affect any prior conclusion
(correlation 0.99998, mean relative diff 0.22% on a sample) — flagged
transparently here, corrected locally in this round's own script.

**Monotonicity**: 0 violations across the full observed MW/rotatable-
bond grid — the transform is exactly monotonic, as designed.
**Saturation**: real but moderate — a 50 Da step adds +0.0147
contribution at low MW (50→100 Da) vs. +0.0104 at high MW (1250→1300
Da), a saturation ratio of 0.71 (29% flatter at the top of the observed
range, not a dramatic cliff).

**Collision analysis — the most direct evidence in this round**:
binned `size_contribution` to 3 decimals; the largest collision buckets
each contain 120-160 molecules landing at (near-)identical
`size_contribution` despite `heteroatom_count` ranging by 6-10 within
the same bucket. Critically, **within several of these exact-score
collision groups, `heteroatom_count` alone still separates hard/easy
locally (AUC up to 0.74, several buckets above 0.6)** — direct,
molecule-level proof that real, currently-discarded signal exists
precisely among molecules the current transform has already collapsed
into indistinguishable scores. Top 20 groups: `results/transform_
audit.json`'s `top_collision_groups`.

## 8-9. Semantic ownership and double-counting (`ownership_audit.py`)

| Descriptor | r vs. `fg_count` | r vs. `fg_contribution` | r vs. other existing signal | Conditional significance |
|---|---|---|---|---|
| `fraction_csp3` | -0.271 | — | **r=0.610 vs. `stereo_contribution`** | Not significant |
| `heteroatom_count` | 0.399 | 0.381 | r=0.332 vs. `mol_wt` | **Significant** |
| `tpsa` | 0.248 | 0.296 | **r=0.709 vs. `heteroatom_count`** | Not significant |

This explains *why* `fraction_csp3` and `tpsa` failed the conditional
test: `fraction_csp3` is substantially collinear with `stereo_
contribution` (already in every baseline) — sp3-rich molecules are
much more likely to have stereocenters; `tpsa` is substantially
collinear with `heteroatom_count` itself (TPSA is computed largely
from heteroatom contributions) — once real signal is present via
either heteroatoms or the existing baseline, TPSA has little left to
add.

`heteroatom_count`'s correlation with FG-related signal is real but
*moderate*, not dominant (r≈0.38-0.40, ~15% shared variance) — and
critically, **every S1-S6 comparison already includes `fg_contribution`
in the F0 baseline**, so `heteroatom_count`'s standalone significance
(p=0.012) is measured *on top of* the existing FG signal, not instead
of it. Not fully redundant with functional-group liability.

**Semantic ownership judgment**: `heteroatom_count` → **size/
composition**. It is a broad compositional *count* (how many
heteroatoms does this molecule have, overall), the same kind of
whole-molecule tally `size_topology` already performs for MW and
rotatable bonds — categorically different from `functional_group_
liability`'s job of detecting *specific* reactive substructure
patterns (Brenk alerts). The two are chemically adjacent (molecules
with more heteroatoms are somewhat more likely to also trip a Brenk
alert) but not the same computation, and the moderate-not-dominant
correlation plus the conditional-significance result both support
treating them as distinct, non-duplicative signals. `fraction_csp3`
and `tpsa` are not pursued further — their ownership question is moot
since neither cleared the conditional-information bar.

## 10. FN archetype coverage (`archetype_coverage.py`)

Reproduced round 22 part 13's exact FN clustering (same F1 columns,
`KMeans(k=6, random_state=0)`) — silhouette reproduced exactly (0.2047
= 0.2047), confirming full determinism before trusting the archetype
labels.

| Archetype | Share of FN | mean `heteroatom_count` |
|---|---|---|
| 0 (simple/flexible) | 32.7% | 4.02 |
| **1 (FG-dense, low-saturated)** | **24.9%** | **6.78** |
| 2 (ring-rich/aromatic) | 19.2% | 3.95 |
| 3 ("classic FM-2": stereo-rich, saturated) | 19.8% | 3.41 |
| 4 (small, complex-stereo) | 2.3% | 2.80 |
| 5 (complex-stereo) | 1.1% | 4.20 |

Global FN mean `heteroatom_count`: 4.55. **`heteroatom_count`'s
signal is strongest in archetype 1 (FG-dense, 24.9% of all FN) — not
the "classic FM-2" stereo archetype (archetype 3), which actually has
the *lowest* mean heteroatom count of the six.** This candidate reaches
a different, complementary slice of the false-negative population than
any stereo-focused candidate would have — real coverage breadth, not a
narrow win confined to one archetype already well-studied this session.

## Decision

**H1 vs. H2 vs. H3**: primarily **H2** (missing-feature, specifically
`heteroatom_count`), with a small, borderline-significant secondary H1
effect (raw MW/RB restoration, p=0.051, ~17% of the total gap). Not a
clean single-hypothesis story, but decisively weighted toward H2.

**A/B/C/D/E**: **B — add one specific new primitive.**
- Not A: S1 alone recovers only ~17% of the F1-vs-F0 gap and is
  borderline-significant at best; "redesign the existing transform"
  is not well supported as the dominant lever.
- Not C: only ONE of the three new descriptors (`heteroatom_count`) is
  independently significant, standalone or conditional — `fraction_
  csp3` and `tpsa` both fail their conditional test, ruling out "a
  small set of features are jointly informative."
- Not D: the H1 (transform) effect is real but small and borderline;
  weighting it equally with H2 would overstate its case.
- Not E: there is a clean, well-evidenced candidate.

## Single next production experiment (not implemented this round)

**Add `num_heteroatoms` as a new input to `size_topology`'s raw burden
formula.** Confirmed present as a chematic-chem 0.13.0 primitive
(`chematic_chem::descriptors::num_heteroatoms`, already verified
existing in round 22 part 13). The specific weight/threshold for this
new term is a separate, later implementation decision — not this
round's job (diagnosis only, per standing discipline).

**Chemical rationale**: heteroatom count is a broad compositional
signal — more N/O/S/halogen atoms generally means more protecting-group
decisions, more chemoselectivity considerations, and more potential
side-reactivity across a synthesis route, independent of raw molecular
weight or flexibility. This is conceptually distinct from `functional_
group_liability`'s specific-reactive-pattern detection (Brenk alerts)
and from `stereochemical_burden`'s stereocenter counting — a
compositional tally that currently has no home in any of the four
components.

**Expected failure mode addressed**: FN archetype 1 specifically
(FG-dense, low-saturation molecules, 24.9% of all false negatives) —
the largest single archetype after the "simple/flexible" one (32.7%,
not addressed by this candidate) — plus a smaller, more diffuse
contribution across the rest of the FN population given the collision
analysis shows real heteroatom-count-driven local separation in
multiple, structurally varied collision buckets, not just one.

**Explicitly NOT proposed** (per instruction, avoiding a vague
multi-feature bundle): "add fsp3 + heteroatoms + tpsa and retune
weights." `fraction_csp3` and `tpsa` did not clear the conditional-
information bar and are not part of this candidate.

## Reproducibility

All randomized steps (`LogisticRegression`, `HistGradientBoostingClassifier`,
`KMeans`) use `random_state=0`. Silhouette for the reproduced FN
clustering matched round 22 part 13's reported value exactly (0.2047).
Scaffold folds are reused verbatim from that round's
`features_with_folds.csv`, not regenerated.

## Outputs

All result files in `results/` are small aggregate JSON (largest:
45KB) — **all committed**, no large per-molecule table generated by
this round (it reuses the prior round's, which stays gitignored there).

## Pipeline

```
python3 decompose.py            # -> results/decomposition_results.json (~4-5 min)
python3 feature_direction.py    # -> results/feature_direction.json
python3 transform_audit.py      # -> results/transform_audit.json
python3 ownership_audit.py      # -> results/ownership_audit.json
python3 archetype_coverage.py   # -> results/archetype_coverage.json
```

## What this round did NOT do (deliberately)

- **No scoring change of any kind** — no weights, thresholds, formulas,
  `RULESET_VERSION`, public report schema, confidence, or new
  component. Analysis only.
- **No TS1/TS2/TS3, no new final holdout.** MPScore development data
  only, per standing sequencing.
- **No implementation of the `heteroatom_count` candidate** — this
  round identifies and narrows the candidate; implementing it (with an
  actual weight, tests, `RULESET_VERSION` bump, CHANGELOG entry) is a
  separate, later decision.
- **No pursuit of `fraction_csp3` or `tpsa`** — both failed the
  conditional-information test this round was specifically designed to
  apply; not carried forward as "maybe later."
