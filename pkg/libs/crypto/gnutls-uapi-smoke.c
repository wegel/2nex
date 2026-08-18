/*
 * Exercises the real installed libgnutls.so through gnutls_global_init()
 * (which loads the system priority file) and gnutls_pkcs11_init() (which
 * loads the deprecated PKCS#11 config), and checks the library's own debug
 * log for the success line each reader prints only after a successful
 * fopen() on the file it chose. Matching on that success line, rather than
 * a bare path substring, tells a real open apart from the "unable to
 * access" line the library also logs (with the same path embedded) when a
 * tier is absent.
 *
 * usage: gnutls-uapi-smoke priority|pkcs11 <expected-path|->
 *        gnutls-uapi-smoke pkcs11 <expected-path> <explicit-path>
 *
 * Pass "-" as <expected-path> to assert that neither reader logged a
 * successful open, which distinguishes the compiled no-file fallback from
 * every tier being present.
 *
 * The three-argument pkcs11 form passes <explicit-path> straight through as
 * gnutls_pkcs11_init()'s non-NULL deprecated_config_file argument, the
 * distinct override callers use for this reader (unlike the priority
 * reader, this reader has no environment-variable override). It proves that
 * argument wins over every /etc, /run, and /usr/lib tier file, which the
 * NULL-only tier walk in the patch must never touch.
 */

#include <gnutls/gnutls.h>
#include <gnutls/pkcs11.h>

#include <stdio.h>
#include <string.h>

static char captured[8192];

static void
log_func (int level, const char *msg)
{
  (void) level;
  size_t used = strlen (captured);
  size_t room = sizeof (captured) - used - 1;
  if (room > 0)
    strncat (captured, msg, room);
}

int
main (int argc, char **argv)
{
  if (argc != 3 && argc != 4)
    {
      fprintf (stderr,
                "usage: %s priority|pkcs11 <expected-path|-> [explicit-path]\n",
                argv[0]);
      return 1;
    }

  const char *mode = argv[1];
  const char *expected = argv[2];
  const char *explicit_path = argc == 4 ? argv[3] : NULL;
  const char *success_prefix;
  char needle[512];

  if (explicit_path != NULL && strcmp (mode, "pkcs11") != 0)
    {
      fprintf (stderr,
                "gnutls uapi smoke: explicit-path only applies to pkcs11 mode\n");
      return 1;
    }

  gnutls_global_set_log_function (log_func);
  gnutls_global_set_log_level (9);

  if (strcmp (mode, "priority") == 0)
    {
      success_prefix = "cfg: loaded system config ";
      gnutls_global_init ();
    }
  else if (strcmp (mode, "pkcs11") == 0)
    {
      success_prefix = "Loading PKCS #11 libraries from ";
      gnutls_global_init ();
      gnutls_pkcs11_init (GNUTLS_PKCS11_FLAG_AUTO, explicit_path);
    }
  else
    {
      fprintf (stderr, "gnutls uapi smoke: unknown mode %s\n", mode);
      return 1;
    }

  if (strcmp (expected, "-") == 0)
    {
      if (strstr (captured, success_prefix) != NULL)
        {
          fprintf (stderr,
                    "gnutls uapi smoke: expected no config file opened, log: %s\n",
                    captured);
          return 1;
        }
    }
  else
    {
      snprintf (needle, sizeof (needle), "%s%s", success_prefix, expected);
      if (strstr (captured, needle) == NULL)
        {
          fprintf (stderr, "gnutls uapi smoke: log did not contain '%s': %s\n",
                    needle, captured);
          return 1;
        }
    }

  gnutls_global_deinit ();
  return 0;
}
