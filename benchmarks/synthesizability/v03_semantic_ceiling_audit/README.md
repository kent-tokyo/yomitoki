# v0.3 Semantic Ceiling Audit (round 22 part 21)

Post-hoc semantic analysis of the ALREADY-FROZEN PaRoutes final holdout
(`experiment/v020-final-holdout`, pre-opening `bac0758`, results
`04ead40`) — not a reopening. No new score is computed on any new
molecule population; nothing here feeds back into a confirmatory
decision or into `v0.2.0-alpha.1`'s scoring. **Zero production diff**
(`git diff main -- src/ tests/ Cargo.toml` returns nothing).

Central question: the high-ring-count route-length correlation collapse
found in round 20 — is it because YOMITOKI is missing structural
information that's actually present in the target molecule (H1/H2), or
because real route depth isn't well-determined by route-free
single-molecule structure at all (H3), or a label/target mismatch (H4)?

## Method summary

- Reused, unmodified: the 9,996-molecule frozen evaluation subset,
  `route_steps` labels, ring/MW/aromatic/stereo/heteroatom strata, and
  `v0.2.0-alpha.1`'s frozen YOMITOKI scores.
- New this round: raw target-only structural descriptors (basic, ring
  topology, stereo, FG-detail — all reused verbatim from
  `information_loss_audit`'s validated RDKit-primitive functions, no new
  chemical perception) + Morgan fingerprint (r=2, 2048 bit), fresh
  Bemis-Murcko scaffold-grouped 5-fold `GroupKFold` over the PaRoutes
  pool specifically (MPScore's fold assignment doesn't apply to a
  different molecule population).
- Probe hierarchy: Ridge regression (linear), `HistGradientBoosting`
  (shallow nonlinear) on raw descriptors, and the same nonlinear model
  on the Morgan fingerprint — all CV'd, all diagnostic-only, no
  hyperparameter search, no new production scoring rule.
- `n5` (a second PaRoutes test set) was checked for the same-target
  multi-route question (§ route multiplicity) — its target pool
  overlaps `n1`'s by only 26% (2,642/10,000). Per the "no new holdout"
  constraint, only that already-opened overlap was used; the other
  7,358 `n5`-only molecules were never touched by anything in this
  round.

## 1. Target-only predictability (sections 3–6)

| Representation | Full set (n=9,996) | Low-ring (n=6,545) | High-ring (n=3,451) | Novel-scaffold (n=7,495) |
|---|---:|---:|---:|---:|
| YOMITOKI (production) | 0.1491 | 0.1614 | **−0.0005** | 0.1227 |
| Raw descriptors, linear | 0.1651 | 0.1642 | 0.0409 | 0.1475 |
| Raw descriptors, nonlinear | 0.1844 | 0.1827 | 0.0880 | 0.1694 |
| Morgan fingerprint, nonlinear | **0.2282** | **0.2458** | **0.1301** | **0.2081** |

(Spearman ρ, scaffold-grouped 5-fold CV throughout; Kendall τ/MAE/rank
-RMSE in `results/predictability_probes.json`.)

**Reading this against section 6's cases**: Morgan fingerprint and raw
descriptors both clearly and consistently beat YOMITOKI's own production
score, on every population slice — this is real evidence of a
**representation gap** (**Case A element**: route-free target structure
has more recoverable signal than YOMITOKI currently captures). But even
the richest representation (Morgan + nonlinear) never crosses 0.25 on
the full set and drops to 0.13 in the high-ring stratum — nowhere near
the "moderate" (0.35) band, let alone "strong" (0.50), from round 20's
own pre-registered correlation bands. That is real evidence of a **low
absolute ceiling** (**Case B element**) for predicting *real historical
route depth* from target-only structure, full stop — not just a
YOMITOKI-specific shortfall.

**Both are true simultaneously.** This is not a clean single-cause
result and is reported as such, not forced into one bucket.

## 2. High-ring collapse decomposition (section 7)

Within the high-ring subset specifically, no finer structural
distinction fares any better — every candidate descriptor's correlation
with `route_steps` inside high-ring is small (|ρ| ≤ 0.067, several not
even significant at n=3,451), versus up to 0.14 in the equivalent
low-ring comparison. YOMITOKI's own score is exactly ρ=−0.0005 within
this subset. **The collapse is not "wrong sub-feature within
high-ring" — no structural descriptor tested here, including several
YOMITOKI doesn't currently use (ring-system count, largest fused
family, bridgeheads, spiro atoms, macrocycle count), predicts route
depth well once you're already in the high-ring population.** This
matches Case C's framing (`results/high_ring_decomposition.json`).

## 3. Matched-pair analysis (section 8)

Structurally near-identical targets (Morgan Tanimoto ≥ 0.6) with
`route_steps` differing by ≥ 2: **37 pairs** out of ~50M possible pairs
in the full set — genuine near-duplicates with large route-length
divergence are a real but rare phenomenon here, not the dominant
pattern (most of the dataset's poor predictability isn't driven by
tons of literal near-duplicate confusion). Mean `route_steps` delta
among the 37: 2.22 steps; mean YOMITOKI difficulty delta: only 0.040 —
**YOMITOKI barely distinguishes these pairs either**, consistent with
the low-ceiling reading rather than "YOMITOKI is specifically fooled by
near-duplicates." Top 50 (here, 37) in `results/matched_pair_analysis.json`.

## 4. Route architecture audit (section 9)

`route_steps` vs. `n_leaf_precursors`: ρ=0.349 (moderate, expected —
more steps typically need more distinct starting materials).
`route_steps` vs. `branching_factor` (convergence proxy): ρ≈0.02,
essentially nothing — **but this test has limited power here**:
`n1`'s own routes are overwhelmingly linear (median branching factor
exactly 1.0, mean 1.01, max 1.67 across the whole subset) — this
specific PaRoutes curation contains very little convergent-synthesis
variation to correlate against target structure in the first place.
Reported as a genuine data limitation of this specific route-metadata
check, not a null finding about convergence's importance in general.

## 5. Stock sensitivity (section 10) — the single strongest piece of evidence this round

Nearest-stock (purchasable-precursor) Morgan similarity vs.
`route_steps`: **ρ = −0.192** overall (more similar to something already
buyable → fewer steps needed, the expected direction), and **stronger,
not weaker, within the high-ring stratum**: ρ = **−0.265** (n=3,451) vs.
−0.144 in low-ring (n=6,545). This is a larger-magnitude correlation
than YOMITOKI's own score achieves anywhere, and larger than raw
descriptors achieve within high-ring specifically (0.041–0.088). A
route-free, single-target-molecule score **structurally cannot see**
which precursors happen to be purchasable — this is direct, quantified
evidence that a meaningful share of what determines real route depth,
especially in the hardest-to-predict regime, lives outside any
possible single-molecule representation. Full detail:
`results/stock_sensitivity.json`.

## 6. Route multiplicity (section 11) — genuine data limitation, disclosed

`n1` and `n5` are substantially disjoint target pools (26% overlap), not
single-route vs. multi-route views of the same targets as their naming
suggested going in. Within the 2,642-molecule overlap, **zero** targets
had more than one route listed per file, and matched `n1`/`n5` step
counts for the same molecule agreed almost perfectly (mean absolute
difference 0.0004, max 1.0 across all 2,642 — effectively one molecule
differing by one step). **This does not support a meaningful
`Var(route_steps | target)` estimate** — the available PaRoutes files
don't provide genuine multi-route data for the same targets. Documented
as a limitation per the round's own contingency instruction, not forced
into a result.

## 7. Ceiling estimate (section 12)

```
YOMITOKI (production)         rho = 0.15
raw descriptors, linear       rho = 0.17
raw descriptors, nonlinear    rho = 0.18
Morgan fingerprint, nonlinear rho = 0.23   <- best target-only ceiling found
```

Per the round's own worked example: this pattern (raw/Morgan clearly
above YOMITOKI, `0.15 → 0.23`) reads as a **real representation gap**,
not just an implementation ceiling — YOMITOKI could plausibly close some
of this by representing structure more richly. But the *absolute*
ceiling even at its best (0.23 full-set, 0.13 high-ring) stays within
round 20's own pre-registered "weak" band and never approaches
"moderate." Combined with the stock-sensitivity finding, the honest
read is: **route-free target-only structure has more signal than
YOMITOKI currently captures, AND that signal's own ceiling for
predicting real historical route depth is low, AND a meaningful,
independently-detectable share of the remainder is genuinely
route-dependent (precursor availability), not recoverable from any
target-only representation at all.**

## 8. Intrinsic vs. route-dependent conceptual split (section 13)

**Intrinsic structural synthesizability** — explainable from the target
molecule alone: topology/ring complexity, size, stereochemical burden,
unusual structural motifs, compositional complexity. This is what
YOMITOKI's four components are already trying to measure, and per this
round's evidence, still have real headroom to measure better (the
raw-descriptor and Morgan gaps above).

**Route-dependent synthesis difficulty** — not determined by the target
alone: precursor availability (this round's clearest positive finding),
protecting-group strategy, convergent-vs-linear route design,
reaction-class availability, chemoselectivity strategy, literature
precedent, historical route choice. None of this is visible to a
route-free, single-molecule score by construction — not a YOMITOKI
implementation gap, a category boundary.

**Should this split go into README/architecture docs?** Not yet, per
explicit instruction (production docs stay untouched this round) — but
this round's evidence makes a reasonably strong case that it eventually
should, since it gives an honest, evidence-backed answer to "why doesn't
YOMITOKI's score match how many steps this really took," which the
project doesn't currently have a documented answer to.

## 9. Product decision (section 14)

**C — two-axis conceptual model**, with a qualification.

Not **A** alone (route-free representation redesign with an unqualified
expectation of matching real route-length prediction) — the ceiling
evidence (§7) says that expectation would be wrong even for a
much-improved intrinsic representation. Not **B** alone (fully narrow
the claim to intrinsic-only, drop route-depth correlation as a goal
entirely) — that would discard the real, still-open representation gap
this round found (Morgan/raw-descriptors clearly beating YOMITOKI, most
of that gap concentrated exactly where a route-free redesign *could*
still help). Not **D** — there is enough convergent, complementary
evidence here (representation gap + low absolute ceiling + independent
stock-availability signal, all triangulating) to support a decision.

**C, qualified**: YOMITOKI's future work should explicitly separate
*intrinsic structural synthesizability* (where representation
improvement is evidenced and worth pursuing — this round's own
concrete lead: close the gap to Morgan/raw-descriptor probes,
especially in the high-ring regime where the gap is largest in relative
terms) from *route-dependent difficulty* (where this round's evidence
says a route-free score has a real, quantified, low ceiling and should
stop implicitly claiming otherwise). Per the Renkin boundary (§15),
this does **not** mean adding retrosynthesis search to YOMITOKI —
route-context evidence stays a separate future tool's job.

## 10. Renkin boundary (section 15)

Held. No retrosynthesis search, route architecture, or stock-dependent
signal was added to YOMITOKI or proposed as a production feature
anywhere in this round. `chematic → yomitoki (intrinsic structural
diagnostics) → renkin (route-dependent planning/evidence)` stays
intact. The stock-sensitivity and route-architecture findings exist
purely to diagnose *why* the ceiling is where it is — not as a pitch
for a new YOMITOKI component.

## 11. No scoring changes / no new holdout (sections 16–17)

Confirmed: zero production diff. `SIZE_WEIGHT_PER_HETEROATOM`, ring L2
aggregation, all `AGGREGATE_WEIGHT_*`, verdict thresholds, and the
confidence formula are untouched. PaRoutes `n1`'s frozen evaluation
subset membership was never altered; `n5` usage was strictly bounded to
its overlap with the already-opened `n1` subset, documented in full
(§6) — no new molecule population was scored or used as evidence
anywhere in this round.
