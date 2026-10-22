#!/usr/bin/env bash
# Dedicated disposable cluster only. Never use this lane for restart trials.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ ${RMDS_DISPOSABLE_CLUSTER:-} == yes ]] || { echo 'ERROR: set RMDS_DISPOSABLE_CLUSTER=yes on a dedicated cluster' >&2; exit 2; }
mkdir -p /tmp/opencode
exec 9>/tmp/opencode/rust-mds-live.lock
flock -n 9 || { echo 'ERROR: another live lane owns the cluster' >&2; exit 1; }
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target-ci-live}
source devcluster/env.sh
[[ -r $CEPH_CONF && -r $RMDS_KEYRING ]] || { echo 'ERROR: live cluster credentials are absent' >&2; exit 1; }
[[ -z $(findmnt -rn -t ceph,fuse.ceph-fuse) ]] || { echo 'ERROR: existing CephFS mounts' >&2; exit 1; }
! pgrep -x ceph-rmds >/dev/null || { echo 'ERROR: existing Rust MDS' >&2; exit 1; }
! pgrep -x ceph-mds >/dev/null || { echo 'ERROR: existing C++ MDS' >&2; exit 1; }
timeout 240 devcluster/start.sh --mds none
timeout 180 cargo run --locked -p osdc --example rados_live
timeout 180 devcluster/newfs.sh
ceph fs get a -f json
timeout 300 scripts/smoke.sh
