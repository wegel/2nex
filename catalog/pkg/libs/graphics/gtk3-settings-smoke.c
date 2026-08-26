/*
 * Read one GtkSettings property from the layered settings.ini files.
 *
 * GtkSettings loads every settings.ini it finds during construction, so
 * creating the object directly exercises the whole search without opening a
 * display. Usage: gtk3-settings-smoke PROPERTY EXPECTED
 */

#include <gtk/gtk.h>
#include <stdio.h>
#include <string.h>

int
main (int argc, char **argv)
{
  GtkSettings *settings;
  char *value = NULL;
  int matched;

  if (argc != 3)
    {
      fprintf (stderr, "usage: %s PROPERTY EXPECTED\n", argv[0]);
      return 2;
    }

  settings = g_object_new (GTK_TYPE_SETTINGS, NULL);
  if (settings == NULL)
    {
      fprintf (stderr, "could not create GtkSettings\n");
      return 2;
    }

  g_object_get (settings, argv[1], &value, NULL);
  matched = (value != NULL && strcmp (value, argv[2]) == 0);
  if (!matched)
    fprintf (stderr, "%s is \"%s\", expected \"%s\"\n",
             argv[1], value != NULL ? value : "(null)", argv[2]);

  g_free (value);
  return matched ? 0 : 1;
}
