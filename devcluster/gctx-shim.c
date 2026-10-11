/*
 * LD_PRELOAD workaround for a Ceph 20.2.0 bug: MDSMap::decode() (libceph-common)
 * checks a log level through the global ceph::global::g_ceph_context, which is NULL
 * in programs that use libcephfs without global_init() (NFS-Ganesha's FSAL_CEPH, the
 * Python "ceph tell mds.*" path). They segfault as soon as they receive an MDS map.
 *
 * - ceph_create() / ceph_create_from_rados() wrappers point g_ceph_context at the
 *   mount's real CephContext (public API ceph_get_mount_context()).
 * - Ganesha dlopen()s its FSAL with RTLD_DEEPBIND, which bypasses those wrappers, so
 *   a dlopen() wrapper also sets g_ceph_context, if still NULL, to a zero-filled
 *   placeholder: every inlined log-level check then reads level 0 and logs nothing.
 *   (Code that really uses g_ceph_context would crash on NULL anyway.)
 *
 * Symbols are looked up with dlsym at run time. Build:
 *   gcc -shared -fPIC -O2 -o gctx-shim.so gctx-shim.c -ldl
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

struct ceph_mount_info;
struct CephContext;

#define GCTX_SYM "_ZN4ceph6global14g_ceph_contextE"

static unsigned char placeholder[1 << 20] __attribute__((aligned(64)));

static void set_gctx(void *lookup, struct ceph_mount_info *cmount)
{
	struct CephContext **g = (struct CephContext **)dlsym(lookup, GCTX_SYM);
	struct CephContext *(*get_ctx)(struct ceph_mount_info *);

	if (!g)
		return;
	if (cmount) {
		get_ctx = (struct CephContext *(*)(struct ceph_mount_info *))
			dlsym(lookup, "ceph_get_mount_context");
		if (get_ctx && (!*g || *g == (struct CephContext *)placeholder))
			*g = get_ctx(cmount);
	} else if (!*g) {
		*g = (struct CephContext *)placeholder;
	}
}

static void *(*real_dlopen)(const char *, int);

void *dlopen(const char *file, int mode)
{
	void *h;

	if (!real_dlopen)
		real_dlopen = (void *(*)(const char *, int))dlsym(RTLD_NEXT, "dlopen");
	h = real_dlopen(file, mode);
	if (h)
		set_gctx(h, NULL);
	return h;
}

/* libcephfs may be loaded RTLD_LOCAL (Python extension): look symbols up through
 * its own handle when the global scope does not have them. */
static void *cephfs_sym(const char *name, void **handle)
{
	void *p = dlsym(RTLD_NEXT, name);

	*handle = RTLD_DEFAULT;
	if (p)
		return p;
	if (!real_dlopen)
		real_dlopen = (void *(*)(const char *, int))dlsym(RTLD_NEXT, "dlopen");
	*handle = real_dlopen("libcephfs.so.2", RTLD_NOW | RTLD_NOLOAD);
	return *handle ? dlsym(*handle, name) : NULL;
}

int ceph_create(struct ceph_mount_info **cmount, const char *const id)
{
	static int (*real)(struct ceph_mount_info **, const char *const);
	static void *handle;
	int r;

	if (!real)
		real = (int (*)(struct ceph_mount_info **, const char *const))
			cephfs_sym("ceph_create", &handle);
	if (!real)
		return -38; /* -ENOSYS */
	r = real(cmount, id);
	if (r == 0)
		set_gctx(handle, *cmount);
	return r;
}

int ceph_create_from_rados(struct ceph_mount_info **cmount, void *cluster)
{
	static int (*real)(struct ceph_mount_info **, void *);
	static void *handle;
	int r;

	if (!real)
		real = (int (*)(struct ceph_mount_info **, void *))
			cephfs_sym("ceph_create_from_rados", &handle);
	if (!real)
		return -38;
	r = real(cmount, cluster);
	if (r == 0)
		set_gctx(handle, *cmount);
	return r;
}
