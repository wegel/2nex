/*
 * LD_PRELOAD interposer for all three readers this package repairs. None of
 * the three has a stable public observable that pins down which tier it
 * actually selected: fips_enabled's public fips-mode flag reads the same
 * "active" value no matter which tier supplied the file, so it cannot tell
 * /run apart from /usr/lib; the enabled hardware feature list is
 * CPU-dependent; and the random.conf flags are internal-only. This wraps
 * the real access() and fopen(), calling through to the libc
 * implementation via dlsym(RTLD_NEXT, ...) so the reader under test still
 * runs unmodified, and prints one line to stderr for every access() or
 * fopen() attempt against a fips_enabled, hwf.deny, or random.conf path,
 * naming the exact path and whether the real call succeeded.
 */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static int
is_watched_path (const char *path)
{
  return path != NULL
         && (strstr (path, "/gcrypt/fips_enabled") != NULL
             || strstr (path, "/gcrypt/hwf.deny") != NULL
             || strstr (path, "/gcrypt/random.conf") != NULL);
}

static FILE *
real_fopen (const char *path, const char *mode)
{
  static FILE *(*fn) (const char *, const char *);

  if (!fn)
    fn = dlsym (RTLD_NEXT, "fopen");

  return fn (path, mode);
}

static int
real_access (const char *path, int mode)
{
  static int (*fn) (const char *, int);

  if (!fn)
    fn = dlsym (RTLD_NEXT, "access");

  return fn (path, mode);
}

FILE *
fopen (const char *path, const char *mode)
{
  FILE *fp = real_fopen (path, mode);

  if (is_watched_path (path))
    fprintf (stderr, "GCRYPT_UAPI_OPEN:%s:%s\n", path, fp ? "ok" : "fail");

  return fp;
}

int
access (const char *path, int mode)
{
  int rc = real_access (path, mode);

  if (is_watched_path (path))
    fprintf (stderr, "GCRYPT_UAPI_ACCESS:%s:%s\n", path, rc == 0 ? "ok" : "fail");

  return rc;
}
