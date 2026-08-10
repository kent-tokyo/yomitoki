"""Score-direction normalization: converts every method's raw score onto a
single convention -- **higher = harder to synthesize** -- matching the
BR-SAScore TS1/TS2/TS3 ground truth (`labels`: 1 = hard/HS, 0 = easy/ES).

Every entry below states where its direction was verified (source code
read + empirical spot-check on aspirin/caffeine/dodecane/pyridine, not
assumed from a paper abstract or a README example). See
../datasets/README.md for the full writeup per method.

Do not add a method here without the same verification discipline.
"""

# method key -> (raw range description, raw direction, transform to
# "higher = harder")
DIRECTIONS = {
    "yomitoki": {
        "raw_range": "[0, 1]",
        "raw_direction": "high = difficulty (already high=hard)",
        "verified": "src/rules.rs DIFFICULTY_* thresholds + report.rs doc comments",
        "transform": lambda raw: raw,
    },
    "sascore": {
        "raw_range": "[1, 10]",
        "raw_direction": "high = hard (verified from rdkit sascorer.py source; aspirin=1.58, caffeine=2.30, dodecane=1.17, pyridine=1.37)",
        "verified": "rdkit.Contrib.SA_Score.sascorer -- read source, ran empirically",
        "transform": lambda raw: raw,
    },
    "brsascore": {
        "raw_range": "[1, 10]",
        "raw_direction": "high = hard (verified from BRSAScore.py source: final line `10 - sascore*9`; aspirin=2.09, caffeine=3.35, dodecane=1.00, pyridine=1.32)",
        "verified": "BRSAScore.BRSAScore.SAScorer -- read source, ran empirically; resolves an earlier apparent PyPI-README discrepancy, see ../datasets/README.md",
        "transform": lambda raw: raw,
    },
}


def normalize(method: str, raw_score: float) -> float:
    """Returns raw_score transformed so higher always means harder."""
    return DIRECTIONS[method]["transform"](raw_score)


def direction_table_markdown() -> str:
    lines = ["| method | raw range | raw direction | benchmark direction |", "|---|---|---|---|"]
    for name, info in DIRECTIONS.items():
        lines.append(f"| {name} | {info['raw_range']} | {info['raw_direction']} | high = hard (unchanged) |")
    return "\n".join(lines)


if __name__ == "__main__":
    print(direction_table_markdown())
