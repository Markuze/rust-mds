#!/usr/bin/env bash
# G2 clean restart / partial G3 controller. Run as root after reviewed readiness.
# Usage: recovery-trial.sh g2|g3 PID MOUNT NEW_ARTIFACT_DIR --owned-mount
#        --rust-filesystem --expected-fscid ID [--binary PATH] [--keyring PATH]
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
if [[ ${RMDS_RECOVERY_READY:-} != yes ]]; then
  echo 'DEPENDENCY: persistence/reconnect integration not enabled; no MDS was stopped'
  exit 77
fi
source "$ROOT/devcluster/env.sh"
exec python3 "$ROOT/scripts/recovery_trial.py" "$@"
