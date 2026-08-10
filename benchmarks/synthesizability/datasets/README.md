# Datasets

No dataset is committed to this repository. Every dataset here is fetched
by a script in `../scripts/`, pinned by SHA256 where a checksum is
meaningful, and documented below with source, license, expected record
count, and the exact preprocessing/exclusion rules applied. `downloaded/`
(the scripts' output directory) is gitignored.

## BR-SAScore TS1 / TS2 / TS3

**Fetch**: `python3 ../scripts/download_brsascore.py ./downloaded`

**Status: usable, with real, disclosed provenance gaps — not silently
substituted, not silently trusted either.**

- **Origin paper**: Chen & Jung, "Estimating the synthetic accessibility
  of molecules with building block and reaction-aware SAScore", *J.
  Cheminform.* 16, 83 (2024).
  [doi:10.1186/s13321-024-00879-0](https://doi.org/10.1186/s13321-024-00879-0)
  (CC-BY 4.0, open access). TS1/TS2/TS3 is itself a naming convention
  BR-SAScore inherited from an earlier paper (DeepSA, Wang et al. 2023,
  [doi:10.1186/s13321-023-00771-3](https://doi.org/10.1186/s13321-023-00771-3)),
  which BR-SAScore's authors re-labeled from scratch using a retrosynthesis
  planner (Retro*): a molecule is "ES" (easy) if a route of ≤10 steps was
  found, "HS" (hard) otherwise.
- **Official repository is dead.**
  `https://github.com/snu-micc/BR-SAScore` returns HTTP 404 as of this
  writing. A [Wayback Machine snapshot from 2024-07-24](http://web.archive.org/web/20240724025815/https://github.com/snu-micc/BR-SAScore)
  confirms the repo existed and listed a `data/` directory, but no
  archived copy of that directory's actual contents was found — so there
  is no way to fetch the authors' own original file, and no official
  checksum exists to verify any copy against.
- **This script downloads from an unofficial third-party mirror**:
  `github.com/awadell1/BR-SAScore` — the only currently-reachable copy
  found. Its authenticity relative to the (vanished) original **cannot be
  cryptographically verified**. Pinned by this project's own SHA256
  (`93621d56743bbe14c97bd0dc846e5f9e48f46ed7e140c57f85cf8fb48bc8e4d9`, of
  the raw `test_set.csv`) so at least *this benchmark's own* reproducibility
  doesn't depend on the mirror staying unchanged — re-running the download
  script later will fail loudly (not silently substitute different data)
  if the mirror's content ever changes.
- **Verified directly** (not assumed from the mirror's own claims): 5,400
  rows, columns `id,smiles,labels,accessibility,dataset`; 0 duplicate
  ids, 0 duplicate SMILES strings; 1/5,400 RDKit-unparseable SMILES
  (excluded during preprocessing, count reported, not silently dropped).
- **A real data-quality wrinkle, resolved, not hidden**: the text
  `accessibility` column (`"es"`/`"hs"`) is blank for 665 of TS2's 1,800
  rows. The numeric `labels` column (`1`=hard/HS, `0`=easy/ES) is fully
  populated for all 5,400 rows and is internally consistent with
  `accessibility` wherever both are present (0 mismatches among the
  4,735 non-blank rows) — **`labels` is used as ground truth**, not the
  partially-populated `accessibility` column. Using `labels`, TS2 turns
  out to be an exactly-balanced 900 hard / 900 easy split.
- **A real discrepancy that could not be resolved**: TS1's class counts
  from this file (745 ES / 1,055 HS) match the paper's own published
  table exactly. **TS2's and TS3's do not**: the paper reports TS2
  858/942 and TS3 810/990; this file's `labels`-based counts are TS2
  900/900 (exactly balanced) and TS3 747/1,052 (1,799 molecules, one
  excluded as unparseable). Whether this reflects a different snapshot
  than the one summarized in the paper's table, a different labeling
  pass, or an alteration in the unofficial mirror could not be
  determined without access to the original repository. Reported here
  rather than silently reconciled or hidden — a reader comparing our
  numbers against the paper's own TS2/TS3 baseline numbers should know
  the underlying label sets are not guaranteed identical.
- **License**: no explicit license is stated for the TS1/TS2/TS3 label
  file itself. The underlying molecule sources (ZINC-15, GDB-17, ChEMBL,
  via the SYBA/Thakkar/GASA papers TS1/TS2/TS3 respectively derive from)
  each carry their own terms. The BR-SAScore *code* (not this data file)
  is MIT-licensed per its [PyPI package](https://pypi.org/project/BRSAScore/)
  metadata.
- **Reconstructing TS1/TS2/TS3 fully from scratch** (independent of any
  mirror) would require obtaining three separate prior papers' datasets
  (SYBA's, Thakkar et al.'s, GASA's) and re-running Retro* with a trained
  one-step retrosynthesis model (GLN, via USPTO reaction data) and a
  building-block stock (eMolecules) — a nontrivial multi-source pipeline,
  not attempted this round; the mirror is used as the pragmatic,
  disclosed-caveats path to a reproducible-enough benchmark.

## MPScore (development set — expert-chemist labels, not a competitor or confirmatory benchmark)

**Status: usable as a development set, with real, disclosed limitations
on rater coverage and domain — investigated by directly downloading and
parsing every file below, not by trusting the source repo's README
prose alone.**

- **Origin**: Bennett, Turcani, Jelfs, "Modelling chemist intuition for
  the synthetic accessibility of porous organic cages",
  `stevenkbennett/synthetic_accessibility_project`. Zenodo DOI badge
  present in the source README, but the README's own paper citation
  parentheses are empty in the pinned commit — no resolvable published
  paper DOI was found this round; cited here by repository, not by
  paper.
- **Pinned to an actual release tag, not a moving HEAD**: tag `1.0.6`,
  commit `b8f4d313c41dbdef1f3404b52d83e9a71673c581`. The repo's default
  branch (`revisions`) has a later commit (`5c43f81c...`, 2023-06-09,
  "Update dependencies") after this tag — deliberately not used, per
  this project's own standing rule (established in round 20 for
  SynRXN) to pin a citable release, not a moving branch tip.
- **License**: MIT, repo-wide (single `LICENSE` file covers code and
  data together — no separate data license). Permits redistribution;
  this project still fetches via script rather than committing the raw
  files, for consistency with every other dataset here and to keep the
  repo lean.
- **What it actually is, verified by parsing every file, not assumed
  from the README's prose**: three chemists independently rated
  molecules as easy-to-synthesize (1) or difficult-to-synthesize (0) via
  a web tool, in the context of screening candidate precursors
  (diamines, trialdehydes, etc.) for porous-organic-cage synthesis. The
  `data/chemist_data/` files are the raw, per-chemist votes:
  `filip.csv` (SMILES-keyed, tab-delimited, 2,008 rows, 1,858 parse to a
  valid unique-after-canonicalization molecule — the source repo does
  not explain the ~150-row gap and it was not investigated further),
  `opinions_becky.json` (InChI-keyed, 10,000 rows, 9,990 parse),
  `opinions_mebriggs.json` (InChI-keyed, 695 rows, all parse). Naming
  and format are inconsistent across the three (one `.csv`, two `.json`,
  one SMILES-keyed, two InChI-keyed) — not normalized upstream, handled
  in `download_mpscore.py`.
- **Rater coverage is very uneven — the single most important caveat**:
  of 10,969 total unique (canonical-SMILES) molecules across the three
  chemists' union, **9,436 (86.0%) were rated by exactly one chemist**,
  1,492 (13.6%) by two, and only **41 (0.4%) by all three**. `becky`
  alone accounts for 9,990 of the 10,969 — the "three expert chemists"
  framing is accurate for methodology but not for coverage: most labels
  in this dataset reflect one person's judgment, not a panel's.
- **`data/training_database.csv` (12,553 rows, the file the source
  README describes as containing the final `chemist_score`) is NOT a
  pre-resolved one-row-per-molecule consensus table — verified, not
  assumed**: 1,544 of its 10,963 unique molecules appear more than once
  across its rows, and **435 of those 1,544 (28%) have mutually
  conflicting `chemist_score` values across their duplicate rows** (e.g.
  one row says 1, another row for the exact same canonical molecule says
  0). This is consistent with the file being one row per *rating event*
  rather than one row per molecule. **This project does not use
  `training_database.csv`'s `chemist_score` column as ground truth.**
  Consensus is instead computed independently from the three raw
  per-chemist files (majority vote where ≥2 raters exist; the single
  rater's vote where only one exists), with `n_raters` and
  `agreement_fraction` preserved per molecule — see
  `download_mpscore.py`.
- **Real disagreement exists among multi-rater molecules, not just
  noise-sized**: among the 1,492 two-rater molecules, agreement is
  71.6% (28.4% disagree); among the 41 three-rater molecules, unanimous
  agreement is 73.2% (11/41 have some disagreement). Per-chemist label
  balance also differs sharply — `becky`: 89%/11% (difficult/easy),
  `filip`: 64%/36%, `mebriggs`: 67%/33% — `becky` is visibly the
  strictest rater, and since she supplies 91% of the raw labels, any
  metric computed on the full set is close to "agreement with becky
  specifically," not "agreement with a panel." Metrics in this project's
  development-set reports are therefore always stratified by `n_raters`.
- **Label direction is the opposite of BR-SAScore's TS1/2/3**: MPScore's
  `chemist_score`/`synthesisable` is **1 = easy, 0 = difficult**.
  BR-SAScore's `labels` is **1 = hard**. Flipped once, explicitly, at
  load time in `download_mpscore.py` (`hard = 1 - chemist_score`), never
  left implicit — getting this backwards silently produces a
  plausible-looking but inverted accuracy number.
- **Domain: narrower research context than TS1/2/3, but not as narrow as
  the paper title alone implies — measured, not inferred**: the study's
  stated purpose is porous-organic-cage precursor screening, but a
  direct functional-group scan of a 2,000-molecule sample of the rated
  pool found only **15.8% carry a primary amine and 16.1% an aldehyde**
  (the classic imine-cage-forming pair) — most rated molecules carry
  neither. Mean molecular weight ≈275, mean ring count ≈2.0, broadly
  comparable in scale to TS1/2/3's populations, not exotic. Read as: a
  broad synthesizability-intuition-rating exercise conducted in the
  context of, but not strictly limited to, cage-precursor candidates.
- **Overlap with TS1/TS2/TS3 — checked directly, not assumed
  negligible**: exact canonical-SMILES overlap is **3 molecules** out of
  10,969 (excluded from the usable pool). Bemis-Murcko scaffold overlap
  on a 2,000-vs-2,000 random sample: 55 of 534 unique MPScore-sample
  scaffolds also appear among the TS-sample's 1,709 scaffolds (~10.3%
  of the sampled MPScore scaffolds) — reported as a statistic, not acted
  on (some scaffold recurrence, e.g. plain benzene/pyridine rings, is
  expected between any two organic-molecule pools and is not evidence of
  leakage by itself). Morgan-fingerprint (r=2, 2048 bit) Tanimoto
  similarity from a 300-molecule MPScore sample to a 1,000-molecule TS
  sample: mean 0.219, median 0.214, **0/300 above 0.7** — no
  near-duplicate risk found.
- **Parse coverage of `training_database.csv` itself**: 12,553/12,553
  rows (100%) RDKit-parseable; the 10,963 figure above is after
  canonical-SMILES deduplication, not a parse failure.
- **Reconstructing SHA256s of the exact files this investigation
  fetched** (pinned commit above; used by `download_mpscore.py` to fail
  loudly if the upstream repo's raw content ever changes):
  - `data/training_database.csv`: `1e20bd5c86f63ef2515895cdeb064899ade70b86d48f117caa79025cf4b6527e`
  - `data/chemist_data/filip.csv`: `4f8d10f95704d9320fab58921370447ded5eaede257bd30454a3fc7e9d2aa1c9`
  - `data/chemist_data/opinions_becky.json`: `d21a4a551045ad9493b56c247a2bb1ca20fb6dd001ba97ad342a0aa3853a96a1`
  - `data/chemist_data/opinions_mebriggs.json`: `2ccb881ad2224f9b55faa776cbccdb0d659b997789b6c1e62d1a360a99cde111`

## SAscore (competitor, not a dataset)

No separate dataset: SAscore is scored directly on the BR-SAScore TS1/TS2/TS3
molecules above (and on any other input) using RDKit's bundled reference
implementation.

- **Origin paper**: Ertl & Schuffenhauer, "Estimation of synthetic
  accessibility score of drug-like molecules based on molecular complexity
  and fragment contributions", *J. Cheminform.* 1, 8 (2009).
  [doi:10.1186/1758-2946-1-8](https://doi.org/10.1186/1758-2946-1-8).
- **Reference implementation used**: `rdkit.Contrib.SA_Score.sascorer`,
  shipped inside the `rdkit` PyPI package itself (verified present at
  `<site-packages>/rdkit/Contrib/SA_Score/{sascorer.py,fpscores.pkl.gz}`
  for `rdkit==2025.9.3`, this project's pinned version). Co-authored by
  Ertl (one of the original paper's authors) and Greg Landrum, but the
  file's own header states it includes "several small modifications to
  the original paper" (a different macrocycle-penalty formula, and an
  added fingerprint-density/symmetry correction not in the paper), and
  reports r²=0.97 against the original PipelinePilot implementation on a
  10k-molecule set — a documented, disclosed divergence from the paper,
  not an exact reproduction of it. No retraining needed: the fragment
  score table ships precomputed.
- **License**: RDKit is BSD-3-Clause; `sascorer.py`'s own header carries a
  Novartis Institutes for BioMedical Research BSD-3-Clause notice.
- **Score direction — verified directly from source, not assumed**:
  `sascorer.calculateScore(mol)` returns a float on **1 (easy) to 10
  (hard)**. Confirmed empirically this round:
  `aspirin=1.58, caffeine=2.30, dodecane=1.17, pyridine=1.37` — all in
  range, ordering matches chemical intuition (the fully-saturated
  building block dodecane scores lowest).
- No independently-reproducible external benchmark exists from the
  original paper itself (only a 40-molecule Novartis-chemist-rated set,
  never publicly released in full and too small for this project's
  purposes regardless) — SAscore is evaluated here only as a competitor
  scored on the BR-SAScore TS1/TS2/TS3 sets, not against its own
  original benchmark.

## BR-SAScore scorer (competitor, not a dataset — see above for the TS1/TS2/TS3 labels)

The BR-SAScore *scores themselves* (as opposed to the TS1/TS2/TS3 ground
truth labels, documented above) are computed live using the official
`BRSAScore` PyPI package, not read from the awadell1 mirror (which
contains no score column at all — only `id,smiles,labels,accessibility,dataset`).

- **Package**: [`BRSAScore`](https://pypi.org/project/BRSAScore/) on
  PyPI, versions 0.1.0/0.1.1, MIT-licensed. Installed with
  `pip install BRSAScore --no-deps` (its declared dependency
  `rdkit-pypi>=2021` is an abandoned PyPI name with no current-platform
  wheels; `--no-deps` avoids the resulting resolution failure and this
  project's own pinned `rdkit==2025.9.3` — the current PyPI name/package —
  is used in its place; verified compatible by actually running it, see
  below).
- **Contains real, self-contained model data**, not LFS pointers or
  stubs — verified by inspecting the installed wheel directly: three
  gzip-compressed serialized fragment-score tables under
  `BRSAScore/pickle/` totaling ~13.2MB
  (`BRScores_uspto_emolecules` 5.9MB, `BScores_emolecules` 5.0MB,
  `RScores_uspto` 2.4MB — building-block and reaction-derived fragment
  score tables).
- **Source inspected directly**: `BRSAScore/BRSAScore.py`'s `SAScorer`
  class is a **derivative of RDKit's own `sascorer.py`** (same file
  header lineage — "peter ertl & greg landrum, september 2013" — same
  `score1 + score2 + score3` structure, same final rescale step), with
  its fragment-score table swapped for one retrained on USPTO reaction
  data + eMolecules building-block stock (the default config:
  `reaction_from='uspto', buildingblock_from='emolecules'`) instead of
  RDKit's generic ChEMBL-derived table. This is the concrete,
  source-level answer to "what does BR-SAScore actually add over
  SAscore": the same structural-complexity formula, plus fragment scores
  informed by which fragments are reachable via real reactions from real
  purchasable building blocks.
- **Score direction — the PyPI README example output previously looked
  inconsistent with the paper's stated 1–10 scale (flagged as an
  unresolved discrepancy in earlier research this round). Resolved here
  by reading `BRSAScore.py`'s source directly**: the final line is
  `sascore = 10 - sascore*9` applied to a value already clamped to
  [0, 1] — i.e. the output range is exactly **1 (easy) to 10 (hard)**,
  matching the paper and matching SAscore's own scale exactly. Confirmed
  empirically: `aspirin=2.09, caffeine=3.35, dodecane=1.00 (exact
  minimum), pyridine=1.32` — all in range 1–10, ordering matches
  chemical intuition, and tracks SAscore's ordering on the same four
  molecules (SAscore: `1.58, 2.30, 1.17, 1.37`) while assigning somewhat
  higher absolute scores, consistent with BR-SAScore's fragment table
  being trained on a narrower, purchasable-building-block-derived
  vocabulary than SAscore's broader ChEMBL-derived one. The earlier
  README example's ~0.87-looking output is not reproduced by this
  package's actual `calculateScore` return value and is not used as the
  basis for this project's direction convention.

## SYBA

**Status: not reproducible from any currently-reachable source — dropped
from this benchmark, evidence below (per the round's brief: an
unreproducible competitor must be reported with evidence, not silently
substituted or silently omitted).**

- **Origin paper**: Voršilák, Kolář, Čmelo, Svozil, "SYBA: Bayesian
  estimation of synthetic accessibility of organic compounds", *J.
  Cheminform.* 12, 35 (2020).
  [doi:10.1186/s13321-020-00439-2](https://doi.org/10.1186/s13321-020-00439-2)
  (CC-BY 4.0).
- **Official repository**: `github.com/lich-uct/syba`, GPL-3.0,
  unmaintained since 2021-01-18. Checked directly this round: its
  pretrained fragment-score resources (`syba/resources/syba.csv.gz`,
  `syba4.csv.gz`) are **Git LFS pointer files, not real data** —
  confirmed by fetching the raw GitHub URL directly:
  ```
  $ curl -sL https://raw.githubusercontent.com/lich-uct/syba/master/syba/resources/syba.csv.gz
  version https://git-lfs.github.com/spec/v1
  oid sha256:32c6b73f0f4bfddffd00d497fa79702d06f575d413f9a74ea92d91ef41cac679
  size 25795242
  ```
  A plain `git clone`/raw-URL fetch (this project's standard reproduction
  path, matching every other dataset/dependency in this directory) yields
  a 133-byte pointer, not the 25.8MB actual file. Retrieving the real
  file would additionally require the repo's configured Git LFS remote to
  still be live and quota-available, which was not verified and is
  outside what "clone a public GitHub repo" normally requires.
- **PyPI has no usable substitute — worse, it is a namespace collision,
  not merely an outdated package.** `pip install syba` installs
  [`SyBA` 0.0.5](https://pypi.org/project/SyBA/), which is **a completely
  unrelated project**: a bacterial-gene/synonymous-codon database tool by
  a different author (`luanrabelo/SyBA`, "Synonymous BActeria"), sharing
  only the acronym. Verified directly by unpacking the installed wheel:
  its single module (`SyBA/SyBA.py`) manages a TSV of gene/protein/strain
  records downloaded from `github.com/luanrabelo/SyBA`, has no
  cheminformatics code at all (no RDKit import, no fragment scoring, no
  molecule handling of any kind), and interactively prompts on `stdin`
  for missing-dependency installs (`input()` calls), making it unsuitable
  for unattended/CI use even if it were the right package. This is not
  "SYBA is outdated" — it is "the PyPI name `syba`/`SyBA` does not refer
  to the synthesizability method at all."
  - Voršilák's own SYBA documents installation via conda
  (`bioconda`/`rdkit` channels), not PyPI. Installing via conda was not
  attempted this round (would pull a separate Python environment/toolchain
  into this benchmark's requirements, and would still hit the same
  underlying Git-LFS-pointer problem for the pretrained score file, since
  conda-forge/bioconda packaging typically vendors the same upstream repo
  contents).
- **Conclusion**: SYBA is excluded from this round's competitor
  comparison. Per the brief's own contingency plan, this is disclosed
  with evidence (above) rather than silently substituted with different
  data or silently dropped without explanation. If SYBA's real pretrained
  score file becomes reachable in the future (e.g. if the upstream LFS
  remote is confirmed reachable with proper LFS tooling, or the authors
  republish it), this section should be revisited — the training-data
  provenance research from earlier this round (ES = 693,353 ZINC15
  molecules; HS = 693,353 molecules from the "Nonpher" molecular-morphing
  algorithm, PMC5359269) remains valid and is preserved here for that
  future attempt.
