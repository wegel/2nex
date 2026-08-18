/*
 * Exercises the real installed libgcrypt.so through the three public entry
 * points that trigger its fixed-path readers: gcry_check_version() runs
 * global_init(), which reads fips_enabled (src/fips.c) and hwf.deny
 * (src/hwfeatures.c) exactly once per process, and gcry_randomize() with
 * GCRY_VERY_STRONG_RANDOM reaches _gcry_rndjent_poll(), which reads
 * random.conf (random/random.c) on its own one-time auto-init. Each mode
 * below is meant to run in a fresh process, since all three readers are
 * gated by static one-time flags inside the library and will not run twice
 * in the same process.
 *
 * usage: libgcrypt-uapi-smoke fips|hwf|random
 *
 * None of the three readers has a stable public observable that pins down
 * which tier it actually selected: fips_enabled's public fips-mode flag
 * reads the same "active" value no matter which tier supplied the file, so
 * on its own it cannot tell /run apart from /usr/lib; the enabled hardware
 * feature list is CPU-dependent; and the random.conf flags are
 * internal-only. The build script therefore always runs every mode under
 * libgcrypt-uapi-interpose.c, an LD_PRELOAD access()/fopen() interposer
 * that records the exact path the library's own reader probed or opened
 * and whether that call succeeded, and asserts on that log.
 *
 * "fips" additionally prints "fips-mode:active" or "fips-mode:inactive" on
 * stdout, taken from the public gcry_fips_mode_active() call. The build
 * script keeps that as secondary evidence alongside the interposer log,
 * not as the sole proof of which tier was selected.
 */

#include <gcrypt.h>

#include <stdio.h>
#include <string.h>

int
main (int argc, char **argv)
{
  if (argc != 2)
    {
      fprintf (stderr, "usage: %s fips|hwf|random\n", argv[0]);
      return 2;
    }

  if (!gcry_check_version (NULL))
    {
      fprintf (stderr, "libgcrypt uapi smoke: gcry_check_version failed\n");
      return 2;
    }

  if (strcmp (argv[1], "fips") == 0)
    {
      printf ("fips-mode:%s\n",
              gcry_fips_mode_active () ? "active" : "inactive");
    }
  else if (strcmp (argv[1], "hwf") == 0)
    {
      /* global_init(), already run by gcry_check_version() above, calls
         _gcry_detect_hw_features(), which is all the hwf.deny reader
         needs; nothing further to trigger here. */
    }
  else if (strcmp (argv[1], "random") == 0)
    {
      unsigned char buf[8];
      gcry_randomize (buf, sizeof (buf), GCRY_VERY_STRONG_RANDOM);
    }
  else
    {
      fprintf (stderr, "libgcrypt uapi smoke: unknown mode %s\n", argv[1]);
      return 2;
    }

  return 0;
}
