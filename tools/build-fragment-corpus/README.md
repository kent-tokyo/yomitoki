# build-fragment-corpus

Builds the fragment-frequency corpus the `fragment_precedent` scoring
component needs — AGENTS.md §5.4 requires a real corpus, not a fabricated
one. Standalone, unpublished build tool: not part of the `yomitoki` crate
published to crates.io (excluded from `cargo publish` automatically, since
it's a nested crate with its own `Cargo.toml`).

**`fragment_precedent`'s scoring formula was confirmed broken in round 16
and redesigned in round 17** as a corpus-relative percentile signal, which
is why this tool computes and stores a `reference_distribution` (see
Output below) — a corpus built with the pre-round-17 tool won't have one
and will be rejected by `FragmentCorpus::load_dir`. Confirmed fixed
end-to-end against the real 200k-molecule corpus for the three documented
target cases (aspirin `0.273 → 0.095`, paracetamol `0.243 → 0.095`,
dodecane `0.068 → 0.000`), with a known corpus-domain-bias caveat for some
structurally-legitimate molecules — see the main crate's `rules.rs`
"Fragment precedent" section for the full before/after data. (The
component itself was named `fragment_rarity` before round 18 — renamed
since it argues difficulty both up and down, not just up.)

## Status

- Pipeline mechanics (parse → filter → dedup → fragment → manifest → hash)
  are implemented and verified end-to-end against a small local fixture:
  identical `fragment_frequencies.json` bytes and `artifact_sha256` across
  repeated runs, despite the manifest's own `generated_at_unix` differing
  each time.
- **ChEMBL 37** (`CC BY-SA 3.0`) is verified reachable via an anonymous
  HTTPS GET from EBI's FTP mirror — no login, no account. This is the first
  real source this pipeline has been validated against.
- **DrugBank Open Data** (`CC0-1.0`) is currently **blocked**: the
  Structures SDF download on `go.drugbank.com/releases/latest#open-data` is
  a JS-driven button (observed "Temporarily unavailable" on 2026-08-09), and
  probing likely REST-shaped download paths returned HTTP 403 even with a
  browser `User-Agent`. This looks like a login-gated download despite the
  dataset itself being CC0. `fetch.sh drugbank` prints the manual steps; it
  does not pretend to have a pinned URL it doesn't have.
- **SureChEMBL** (`CC BY 4.0`) is researched (see
  `tasks/upstream_and_corpus_research.md`, gitignored) but not yet wired
  into `fetch.sh`.
- **Unresolved**: whether ChEMBL's CC BY-SA 3.0 data can be combined with
  SureChEMBL's CC BY 4.0 data under a single CC BY-SA 3.0-labeled combined
  artifact. One-way compatibility between CC license *versions* is not
  automatic and has not been verified — don't rely on it until it is. Not a
  blocker today: only ChEMBL is wired up so far.

## Reproducing a build

```sh
./fetch.sh chembl          # downloads + decompresses data/raw/chembl_37_chemreps.txt(.gz)
cargo run --release -- \
  --output data/out \
  --source "ChEMBL 37|CC-BY-SA-3.0|https://www.ebi.ac.uk/chembl/|data/raw/chembl_37_chemreps.txt" \
  --corpus-domain-name "ChEMBL-37" --corpus-domain "bioactivity" \
  --corpus-synthesis-focused false \
  --corpus-domain-description "Bioactive compound reference corpus (drug-like molecules tested for biological activity); not a synthesis-focused precedent corpus." \
  --delimiter tab --smiles-column 1 --name-column 0 --title-line \
  --radius 2
```

**`--radius` takes exactly one value** (default `2`, ECFP4-equivalent),
not a list — `chematic::fp::morgan_fp_counts(mol, radius)` is cumulative,
it returns iterations `0..=radius` merged into one map (confirmed by
reading `chematic-fp`'s source), matching RDKit's own
`GetMorganFingerprint` semantics, so multiple radii would just store the
same underlying fragment hashes redundantly under different keys (round
15's finding — a `--radii 0,1,2` list flag used to exist, removed in round
17 since the molecule-level reference distribution is inherently
single-radius too).

Add `--limit N` to smoke-test on the first N kept molecules instead of the
whole corpus (parsing is the expensive part — `--limit` still reads the
whole input file first, see the `ponytail:` note in `src/main.rs`).

`src/bin/query.rs` is a small diagnostic tool for checking whether a built
corpus actually discriminates: `cargo run --release --bin query -- --corpus
<dir> "<SMILES>" --radii 2` reports document-frequency stats (min/max/mean)
for a molecule's fragments against a built corpus (its own `--radii` flag
still accepts a list for ad hoc querying — it's read-only, doesn't affect
what a corpus contains). Used to validate the corpus against `rules.rs`'s
own documented false positives (aspirin, long unbranched chains) — see
`tasks/upstream_and_corpus_research.md`'s Part 5 for the actual numbers.

Multiple `--source` flags in one invocation are deduplicated by canonical
SMILES *across all of them*, so a molecule present in both ChEMBL and a
future SureChEMBL/DrugBank source is only counted once — this only works
correctly if every source you want deduplicated together is passed in the
same invocation.

`data/` (downloads and pipeline output) is gitignored — nothing here is
committed. Re-running `fetch.sh` + `build-fragment-corpus` regenerates it
byte-for-byte (see `manifest.json`'s `artifact_sha256`, computed over
`fragment_frequencies.json` only, not over the manifest itself — the
manifest's `generated_at_unix` is real wall-clock time and legitimately
differs run to run).

## Output

- `fragment_frequencies.json` — for each fragment hash produced by
  `chematic::fp::morgan_fp_counts`, the number of distinct molecules
  containing that fragment at least once (document frequency, not a raw
  atom-environment count).
- `manifest.json` — per-source provenance (name, license, URL, input file
  sha256, records read/parsed-error/filtered/kept/duplicate), the exact
  filtering/dedup rules applied, radius used, `chematic_version`,
  `tool_version`, `artifact_sha256`, plus (round 17, for reproducibility):
  - `yomitoki_ruleset_version_at_build` — `yomitoki::RULESET_VERSION`
    current when this corpus was built (correlates a corpus's numbers with
    which `fragment_precedent` formula produced them).
  - `fragment_definition_version` / `fragment_definition` — a version tag
    plus a human-readable description of exactly how a "fragment" is
    defined and hashed, so `fragment_frequencies.json` is interpretable
    without reading this tool's Rust source.
  - `mean_document_frequency` / `median_document_frequency` — corpus-wide
    summary of the same per-molecule statistic `fragment_precedent::compute`
    computes at inference time.
  - `reference_distribution_definition` — human-readable description of
    the grid below.
  - `reference_distribution` — a 1001-point quantile grid (p = 0.000,
    0.001, ..., 1.000) of this corpus's own molecule-level
    mean-document-frequency distribution — computed in a second pass,
    after the frequency table is complete, over every kept molecule.
    `FragmentCorpus::percentile_rank` uses this to convert a query
    molecule's mean document frequency into an empirical percentile
    against this exact corpus (see `rules.rs`'s "Fragment precedent"
    section for why an absolute scale doesn't work).
  - `reference_distribution_version` — version tag for how the reference
    distribution is computed, independent of `tool_version` (round 18,
    mirroring `fragment_definition_version` above).
  - `reference_distribution_quantiles` — named q01/q05/q10/q25/q50/q75/
    q90/q95/q99 convenience subset of the grid above.
  - `corpus_domain` (round 18, **required** — see `--corpus-domain-*`
    below) — `source_name`/`domain`/`synthesis_focused`/`description`:
    what chemical space this corpus represents, so a report produced
    against it (`Provenance.fragment_corpus`) can distinguish "rare in
    this corpus" from "hard to synthesize." A provenance declaration the
    builder asserts, not something this tool verifies.

## Not yet done

- A decision on final corpus target size. Measured (real, not
  extrapolated-from-one-point) scaling at 5,000 / 50,000 / 200,000
  molecules: 29,165 / 94,848 / 202,993 distinct fragments, sublinear
  growth (~`N^0.52`), gzip compression holding steady at ~8.5×. 200k
  already produces a real discriminating signal (see
  `tasks/upstream_and_corpus_research.md` Part 5) at 2.47 MB compressed —
  comfortably under crates.io's 10 MB default limit. A full, un-capped run
  over all of ChEMBL 37 (~2.9M compounds) was not attempted — extrapolating
  the same growth curve lands right at the 10 MB boundary, genuinely
  uncertain either way without actually measuring it.
- SureChEMBL wiring in `fetch.sh`.
- DrugBank's download blocker (see Status above) — needs either the site
  coming back, or a manual authenticated download.
- The `yomitoki-data` license-split packaging (AGENTS.md §5.4's suggested
  `yomitoki-core`/`yomitoki-models`/`yomitoki-data` split) that would
  actually ship a built corpus artifact. This tool only builds the artifact
  locally; it doesn't yet decide where a shipped copy would live or under
  what license file.
- Emailing ChEMBL (`chembl-help@ebi.ac.uk`) about the ShareAlike-on-derived-
  aggregates question noted above, if a shipped artifact ever depends on the
  answer.
