#!/usr/bin/env bash
# Offline Ceph oracle. Run from crates/osdc/testdata; no cluster access.
set -euo pipefail
file=$(mktemp /tmp/opencode/pgnls.XXXXXX)
trap 'rm -f "$file"' EXIT
ceph-dencoder type pg_nls_response_t select_test 3 encode export "$file"
xxd -p "$file" > pgnls_response3.hex
