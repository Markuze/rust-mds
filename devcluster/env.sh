# shellcheck shell=bash
# Source me:  source /mnt/workspace/rust-mds/devcluster/env.sh
# Exports the paths and addresses of the dev cluster for tools and tests.
export DEVCLUSTER=/mnt/workspace/rust-mds/devcluster
export DEVCLUSTER_DATA=/mnt/workspace/devcluster-data
export CEPH_CONF=$DEVCLUSTER_DATA/ceph.conf
# client.admin keyring (the ceph CLI finds it through CEPH_CONF anyway).
export CEPH_KEYRING=$DEVCLUSTER_DATA/keyring/ceph.client.admin.keyring
# client.admin secret only (base64), for mount -o secretfile=
export CEPH_ADMIN_SECRET=$DEVCLUSTER_DATA/keyring/client.admin.secret
# keyring of the Rust MDS daemon, entity mds.rust
export RMDS_KEYRING=$DEVCLUSTER_DATA/keyring/ceph.mds.rust.keyring
export RMDS_NAME=mds.rust
export MON_V1=127.0.0.1:6789
export MON_V2=127.0.0.1:3300
export CEPH_FS_NAME=a
export CEPH_LOG_DIR=$DEVCLUSTER_DATA/log
# LD_PRELOAD this for libcephfs users that crash on MDS maps with Ceph 20.2.0,
# e.g.  LD_PRELOAD=$GCTX_SHIM ceph tell mds.a session ls   (see devcluster/gctx-shim.c)
export GCTX_SHIM=$DEVCLUSTER_DATA/lib/gctx-shim.so
FSID=$(cat "$DEVCLUSTER_DATA/fsid" 2>/dev/null || echo unknown)
export FSID
