# build-fragment-corpus

Builds the fragment-frequency corpus the (not yet implemented)
`fragment_rarity` scoring component needs — AGENTS.md §5.4 requires a real
corpus, not a fabricated one, so this tool exists before that component
does. Standalone, unpublished build tool: not part of the `yomitoki` crate
published to crates.io (excluded from `cargo publish` automatically, since
it's a nested crate with its own `Cargo.toml`).

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
  --delimiter tab --smiles-column 1 --name-column 0 --title-line
```

Add `--limit N` to smoke-test on the first N kept molecules instead of the
whole corpus (parsing is the expensive part — `--limit` still reads the
whole input file first, see the `ponytail:` note in `src/main.rs`).

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

- `fragment_frequencies.json` — for each `(radius, fragment_hash)` pair
  produced by `chematic::fp::morgan_fp_counts`, the number of distinct
  molecules containing that fragment at least once (document frequency, not
  a raw atom-environment count).
- `manifest.json` — per-source provenance (name, license, URL, input file
  sha256, records read/parsed-error/filtered/kept/duplicate), the exact
  filtering/dedup rules applied, radii used, `chematic_version`,
  `tool_version`, and `artifact_sha256`.

## Not yet done

- Full, un-capped run over all of ChEMBL 37 (~2.4M compounds) — only
  smoke-tested with `--limit` so far. A full run's wall-clock cost hasn't
  been measured.
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
