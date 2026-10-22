#!/usr/bin/env bash
# Offline unit and golden checks. Missing optional capture keys remain visible.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v ceph-dencoder >/dev/null || { echo 'ERROR: install ceph-common for golden checks' >&2; exit 1; }
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target-ci-offline}
status=0
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests || status=1
cargo fmt --all -- --check || status=1
cargo clippy --workspace --all-targets -- -D warnings || status=1
# --nocapture exposes tests which return early when optional fixtures/keys are absent.
cargo test --workspace --locked -- --nocapture || status=1
exit "$status"
