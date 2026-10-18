#!/usr/bin/env bash
# Verify that a release is self-contained and never ships the archival ref/
# tree. Run from the repository root.
set -euo pipefail

echo "== packaged files =="
cargo package --list --allow-dirty > /tmp/digger-package-files.txt
if grep -qE '^(\./)?ref/' /tmp/digger-package-files.txt; then
    echo "ERROR: ref/ would be included in the package" >&2
    exit 1
fi
if grep -qE '^(\./)?digger\.sco$' /tmp/digger-package-files.txt; then
    echo "ERROR: local high-score file would be included in the package" >&2
    exit 1
fi
for required in src/main.rs src/lib.rs assets/vga.spr assets/vgatext.spr assets/vtitle.bmp; do
    if ! grep -qx "$required" /tmp/digger-package-files.txt; then
        echo "ERROR: packaged file missing: $required" >&2
        exit 1
    fi
done
echo "package is self-contained; ref/ excluded"

echo "== tests =="
cargo test
echo "OK"
