# Competitors

Competitor-specific dependencies live here, not in `../requirements.txt`,
since they're only needed to actually run a competitor score, not for the
core harness/metrics code or the CI smoke test. Full provenance
(paper, license, score direction, reproducibility verdict) for each is in
`../datasets/README.md` — this file only covers "how do I actually install
and run it."

## SAscore — usable

```
pip install rdkit==2025.9.3   # already in ../requirements.txt
python3 ../scripts/run_sascore.py <input.smi> <output.jsonl>
```

Uses RDKit's bundled `rdkit.Contrib.SA_Score.sascorer` directly — no
extra download, no extra dependency beyond `rdkit` itself.

## BR-SAScore — usable, with one install caveat

```
pip install BRSAScore --no-deps   # see caveat below
python3 ../scripts/run_brsascore.py <input.smi> <output.jsonl>
```

**Caveat**: `BRSAScore`'s declared dependency is `rdkit-pypi>=2021`, an
abandoned PyPI package name with no wheels for current Python/platform
combinations — a plain `pip install BRSAScore` fails dependency
resolution. Install with `--no-deps`; this project's own pinned
`rdkit==2025.9.3` (the current PyPI package name) satisfies what
`BRSAScore` actually imports (`from rdkit import Chem`, etc.) at runtime.
Verified working end-to-end this round, not merely assumed compatible.

## SYBA — not reproducible, excluded

Both the official repo's pretrained score file (Git LFS pointer, not
real data) and the PyPI `syba`/`SyBA` package (an unrelated
bacterial-gene-database tool, not the synthesizability method) fail to
provide usable, runnable SYBA scores. No install/run instructions are
given because none currently work. Full evidence in
`../datasets/README.md`'s SYBA section. SYBA is excluded from this
benchmark's accuracy comparison; this is disclosed, not silently
patched over.
