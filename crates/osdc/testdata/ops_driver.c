/* Drive a scripted series of RADOS operations through librados (the C++ Objecter) so that
 * their MOSDOp/MOSDOpReply messages can be captured as golden data for the osdc crate.
 * Build: gcc -I<ceph>/src/include ops_driver.c /usr/lib/x86_64-linux-gnu/librados.so.2
 * Run:   ops_driver <ceph.conf> <pool>
 * Each step prints "step N ..." with the result so the capture can be matched up. */
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "rados/librados.h"

static int step = 0;
#define CHECK(what, r) do { printf("step %d %s r=%d\n", ++step, what, (int)(r)); fflush(stdout); usleep(200000); } while (0)

int main(int argc, char **argv) {
  rados_t c;
  rados_ioctx_t io;
  if (argc < 3) return 2;
  if (rados_create(&c, "admin") < 0) return 1;
  rados_conf_read_file(c, argv[1]);
  if (rados_connect(c) < 0) { fprintf(stderr, "connect failed\n"); return 1; }
  if (rados_ioctx_create(c, argv[2], &io) < 0) { fprintf(stderr, "ioctx failed\n"); return 1; }

  CHECK("write_full obj1", rados_write_full(io, "obj1", "hello world", 11));
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_create(op, LIBRADOS_CREATE_EXCLUSIVE, NULL);
    rados_write_op_setxattr(op, "parent", "abc", 3);
    const char *keys[] = {"k1", "k2"};
    const char *vals[] = {"v1", "value2"};
    size_t klens[] = {2, 2}, vlens[] = {2, 6};
    rados_write_op_omap_set2(op, keys, vals, klens, vlens, 2);
    CHECK("create excl + setxattr + omap_set obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
  }
  {
    rados_read_op_t op = rados_create_read_op();
    uint64_t size; time_t mtime; int r1, r2, r3, r4, r5;
    char buf[64]; size_t bytes_read;
    rados_xattrs_iter_t xit;
    rados_omap_iter_t oit, hit;
    unsigned char more;
    rados_read_op_stat(op, &size, &mtime, &r1);
    rados_read_op_read(op, 0, sizeof(buf), buf, &bytes_read, &r2);
    rados_read_op_getxattrs(op, &xit, &r3);
    rados_read_op_omap_get_vals2(op, "", "", 10, &oit, &more, &r4);
    rados_read_op_omap_get_vals_by_keys(op, (const char *[]){"k2"}, 1, &hit, &r5);
    CHECK("stat + read + getxattrs + omap_get_vals + omap_get_vals_by_keys obj1", rados_read_op_operate(op, io, "obj1", 0));
    rados_release_read_op(op);
  }
  {
    rados_read_op_t op = rados_create_read_op();
    uint64_t size; time_t mtime; int r1, r3, r4;
    rados_xattrs_iter_t xit;
    rados_omap_iter_t oit;
    unsigned char more;
    rados_read_op_stat(op, &size, &mtime, &r1);
    rados_read_op_getxattrs(op, &xit, &r3);
    rados_read_op_omap_get_vals2(op, "k1", "", 10, &oit, &more, &r4);
    CHECK("stat + getxattrs + omap_get_vals(after k1) obj2", rados_read_op_operate(op, io, "obj2", 0));
    rados_release_read_op(op);
  }
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_cmpxattr(op, "parent", LIBRADOS_CMPXATTR_OP_EQ, "abc", 3);
    rados_write_op_write(op, "XYZ", 3, 4);
    CHECK("cmpxattr eq + write obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
  }
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_cmpxattr(op, "parent", LIBRADOS_CMPXATTR_OP_GT, "zzz", 3);
    rados_write_op_write(op, "NO", 2, 0);
    CHECK("cmpxattr gt (fails) + write obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
  }
  {
    uint64_t ver = rados_get_last_version(io);
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_assert_version(op, ver);
    rados_write_op_truncate(op, 2);
    CHECK("assert_version + truncate obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
    op = rados_create_write_op();
    rados_write_op_assert_version(op, ver);
    rados_write_op_truncate(op, 1);
    CHECK("assert_version (stale) + truncate obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
  }
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_omap_rm_keys2(op, (const char *[]){"k1"}, (size_t[]){2}, 1);
    rados_write_op_rmxattr(op, "parent");
    rados_write_op_append(op, "tail", 4);
    rados_write_op_zero(op, 0, 1);
    CHECK("omap_rm_keys + rmxattr + append + zero obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
  }
  {
    rados_read_op_t op = rados_create_read_op();
    rados_omap_iter_t it; int r;
    rados_read_op_omap_get_keys2(op, "", 100, &it, NULL, &r);
    CHECK("omap_get_keys obj2", rados_read_op_operate(op, io, "obj2", 0));
    rados_release_read_op(op);
  }
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_omap_clear(op);
    CHECK("omap_clear obj2", rados_write_op_operate2(op, io, "obj2", NULL, 0));
    rados_release_write_op(op);
  }
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_create(op, LIBRADOS_CREATE_EXCLUSIVE, NULL);
    CHECK("create excl obj1 (exists)", rados_write_op_operate2(op, io, "obj1", NULL, 0));
    rados_release_write_op(op);
  }
  {
    rados_write_op_t op = rados_create_write_op();
    rados_write_op_create(op, LIBRADOS_CREATE_IDEMPOTENT, NULL);
    CHECK("create non-excl obj1", rados_write_op_operate2(op, io, "obj1", NULL, 0));
    rados_release_write_op(op);
  }
  {
    uint64_t size; time_t mtime;
    CHECK("stat missing", rados_stat(io, "missing", &size, &mtime));
  }
  CHECK("remove missing", rados_remove(io, "missing"));
  {
    /* writes with a self-managed snap context */
    uint64_t s1, s2;
    rados_ioctx_selfmanaged_snap_create(io, &s1);
    rados_ioctx_selfmanaged_snap_create(io, &s2);
    rados_snap_t snaps[] = {s2, s1};
    rados_ioctx_selfmanaged_snap_set_write_ctx(io, s2, snaps, 2);
    CHECK("write with snapc obj1", rados_write(io, "obj1", "SNAP", 4, 0));
    CHECK("remove with snapc obj2", rados_remove(io, "obj2"));
    rados_ioctx_selfmanaged_snap_set_write_ctx(io, 0, NULL, 0);
  }
  rados_ioctx_set_namespace(io, "ns1");
  CHECK("write_full ns1/obj3", rados_write_full(io, "obj3", "namespaced", 10));
  {
    char buf[32];
    CHECK("read ns1/obj3", rados_read(io, "obj3", buf, sizeof(buf), 0));
  }
  rados_ioctx_set_namespace(io, "");
  rados_ioctx_locator_set_key(io, "lockey");
  CHECK("write_full key=lockey obj4", rados_write_full(io, "obj4", "keyed", 5));
  rados_ioctx_locator_set_key(io, NULL);
  {
    char big[20000];
    memset(big, 'b', sizeof(big));
    CHECK("write_full big obj5", rados_write_full(io, "obj5", big, sizeof(big)));
    CHECK("read big obj5", rados_read(io, "obj5", big, sizeof(big), 0));
  }
  CHECK("remove obj1", rados_remove(io, "obj1"));
  rados_ioctx_destroy(io);
  rados_shutdown(c);
  return 0;
}
