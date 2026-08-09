#!/usr/bin/env bash
# Downloads pinned, versioned fragment-corpus source snapshots into
# data/raw/, verifying each against a checked-in sha256 (computed after the
# first successful download of this exact snapshot — see the *_SHA256
# variables below). Never points at a "latest" symlink for anything that
# isn't itself already a dated/versioned release.
#
# Usage: ./fetch.sh [chembl|drugbank|all]  (default: all)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAW_DIR="$SCRIPT_DIR/data/raw"
mkdir -p "$RAW_DIR"

USER_AGENT="yomitoki-corpus-builder/0.1 (https://github.com/kent-tokyo/yomitoki)"

# ---------------------------------------------------------------------------
# ChEMBL 37 chemical representations (CC BY-SA 3.0; EBI FTP, anonymous
# HTTPS, no login required — verified reachable 2026-08-09).
# ---------------------------------------------------------------------------
CHEMBL_URL="https://ftp.ebi.ac.uk/pub/databases/chembl/ChEMBLdb/latest/chembl_37_chemreps.txt.gz"
CHEMBL_GZ="$RAW_DIR/chembl_37_chemreps.txt.gz"
CHEMBL_TXT="$RAW_DIR/chembl_37_chemreps.txt"
# Pinned to chembl_37 explicitly, not "latest" — "latest" in the URL path
# above is EBI's own alias for the chembl_37 directory as of this writing;
# re-verify it still resolves to chembl_37 before relying on it long-term.
# Verified 2026-08-09 against https://ftp.ebi.ac.uk/pub/databases/chembl/ChEMBLdb/latest/chembl_37_chemreps.txt.gz
CHEMBL_GZ_SHA256="ea6181ce8dc7af41974e35b92e1febb0c9dcbe2c62f7ccc4a5d983ac19f696e7"

fetch_chembl() {
    echo "== ChEMBL 37 chemreps (CC BY-SA 3.0) =="
    if [[ -f "$CHEMBL_GZ" ]]; then
        echo "already downloaded: $CHEMBL_GZ"
    else
        curl -A "$USER_AGENT" --fail --show-error --progress-bar \
            -o "$CHEMBL_GZ" "$CHEMBL_URL"
    fi
    actual_sha256="$(shasum -a 256 "$CHEMBL_GZ" | cut -d' ' -f1)"
    echo "sha256: $actual_sha256"
    if [[ "$CHEMBL_GZ_SHA256" != "__FILL_IN_AFTER_FIRST_DOWNLOAD__" \
        && "$actual_sha256" != "$CHEMBL_GZ_SHA256" ]]; then
        echo "error: sha256 mismatch — EBI's 'latest' pointer may have moved" >&2
        echo "  expected: $CHEMBL_GZ_SHA256" >&2
        echo "  actual:   $actual_sha256" >&2
        exit 1
    fi
    gunzip -kf "$CHEMBL_GZ"
    echo "decompressed: $CHEMBL_TXT"
    echo
    echo "Next: run build-fragment-corpus with:"
    echo "  --source \"ChEMBL 37|CC-BY-SA-3.0|https://www.ebi.ac.uk/chembl/|$CHEMBL_TXT\" \\"
    echo "  --delimiter tab --smiles-column 1 --name-column 0 --title-line"
}

# ---------------------------------------------------------------------------
# DrugBank Open Data structures SDF (CC0-1.0) — currently BLOCKED, not
# automatable from this script. go.drugbank.com/releases/latest#open-data
# shows the Structures download as a JS-driven button, not a static URL, and
# was observed "Temporarily unavailable" as of 2026-08-09; probing likely
# REST-shaped paths (e.g. /releases/<version>/downloads/all-open-structures)
# returned HTTP 403 even with a browser User-Agent, consistent with a
# login-gated download despite the dataset itself being CC0. This is a
# manual/account-gated step, not something fetch.sh can pin a URL for
# without fabricating one.
# ---------------------------------------------------------------------------
fetch_drugbank() {
    echo "== DrugBank Open Data structures (CC0-1.0) =="
    echo "BLOCKED: no stable, unauthenticated download URL found as of"
    echo "2026-08-09 — go.drugbank.com/releases/latest#open-data serves the"
    echo "Structures SDF behind a JS download button (page showed"
    echo "'Temporarily unavailable'); likely REST-shaped URLs return 403."
    echo "Manual step: log in (free account) at https://go.drugbank.com,"
    echo "download the Open Data Structures SDF from that page, and place it"
    echo "at: $RAW_DIR/drugbank_open_structures.sdf"
    if [[ -f "$RAW_DIR/drugbank_open_structures.sdf" ]]; then
        echo "found: $RAW_DIR/drugbank_open_structures.sdf ($(shasum -a 256 "$RAW_DIR/drugbank_open_structures.sdf" | cut -d' ' -f1))"
    fi
}

case "${1:-all}" in
    chembl) fetch_chembl ;;
    drugbank) fetch_drugbank ;;
    all)
        fetch_chembl
        echo
        fetch_drugbank
        ;;
    *)
        echo "usage: $0 [chembl|drugbank|all]" >&2
        exit 2
        ;;
esac
