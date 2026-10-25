#!/usr/bin/env bash
# Offline-reviewed readiness is separate from authorization for any live run.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/devcluster/env.sh"
exec python3 "$ROOT/scripts/g3_trial.py" "$@"
