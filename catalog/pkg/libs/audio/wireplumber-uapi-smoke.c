/*
 * Opens a named WirePlumber configuration through the real installed
 * WpConf reader (the same wp_conf_open() path the daemon uses at startup,
 * covering both the single main file and its .conf.d fragment directory)
 * and checks a given section's raw JSON text against one required
 * substring and zero or more forbidden substrings.
 *
 * usage: wireplumber-uapi-smoke <config-name> <section> <require|-> [forbid...]
 *
 * <require> is a literal substring that must appear in the section's text.
 * Pass "-" to assert that the section is absent instead, in which case no
 * <forbid> arguments are allowed. Each <forbid> is a literal substring that
 * must NOT appear in the section's text, letting a same-basename collision
 * test prove that a shadowed lower-tier file's keys did not leak through.
 */

#include <wp/wp.h>

#include <stdio.h>
#include <string.h>

int
main (int argc, char **argv)
{
  if (argc < 4) {
    fprintf (stderr,
        "usage: %s <config-name> <section> <require|-> [forbid...]\n",
        argv[0]);
    return 1;
  }

  const gchar *name = argv[1];
  const gchar *section_name = argv[2];
  const gchar *require = argv[3];
  gchar **forbid = argv + 4;
  gint n_forbid = argc - 4;

  if (strcmp (require, "-") == 0 && n_forbid != 0) {
    fprintf (stderr,
        "wireplumber uapi smoke: forbid arguments make no sense with '-'\n");
    return 1;
  }

  wp_init (WP_INIT_ALL);

  g_autoptr (GError) error = NULL;
  g_autoptr (WpConf) conf = wp_conf_new_open (name, NULL, &error);
  if (!conf) {
    fprintf (stderr, "wireplumber uapi smoke: failed to open '%s': %s\n",
        name, error ? error->message : "unknown error");
    return 1;
  }

  g_autoptr (WpSpaJson) section = wp_conf_get_section (conf, section_name);

  if (strcmp (require, "-") == 0) {
    if (section) {
      g_autofree gchar *text = wp_spa_json_to_string (section);
      fprintf (stderr,
          "wireplumber uapi smoke: section '%s' should be absent, found: %s\n",
          section_name, text);
      return 1;
    }
    return 0;
  }

  if (!section) {
    fprintf (stderr, "wireplumber uapi smoke: section '%s' was not found\n",
        section_name);
    return 1;
  }

  g_autofree gchar *text = wp_spa_json_to_string (section);
  if (!text || !strstr (text, require)) {
    fprintf (stderr,
        "wireplumber uapi smoke: section '%s' did not contain '%s': %s\n",
        section_name, require, text ? text : "(null)");
    return 1;
  }

  for (gint i = 0; i < n_forbid; i++) {
    if (strstr (text, forbid[i])) {
      fprintf (stderr,
          "wireplumber uapi smoke: section '%s' leaked forbidden '%s': %s\n",
          section_name, forbid[i], text);
      return 1;
    }
  }

  return 0;
}
