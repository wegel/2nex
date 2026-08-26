/*
 * Exercises the real, newly built libappstream UAPI-config reader this
 * package patches, through its normal public entry points rather than a
 * reimplemented parser.
 *
 * argv[1] is a repository origin string. This smoke builds a fresh
 * #AsComponent, gives it a package name (so
 * as_utils_get_component_bundle_kind(), src/as-utils.c, reports
 * AS_BUNDLE_KIND_PACKAGE) and no license, then calls the public
 * as_component_is_floss() (src/as-component.c). With no license set,
 * as_license_is_free_license (NULL) is FALSE, so as_component_is_floss()
 * falls through to the internal as_context_os_origin_is_free(), which
 * calls the patched as_context_ensure_os_config_loaded() and matches
 * argv[1] against the FreeRepos glob list loaded from whichever
 * configuration file that patched selection chose. Printing the result
 * proves which file's FreeRepos value actually took effect, not merely
 * that some file was named in a debug line.
 *
 * A fresh AsComponent is used, and each invocation of this program is a
 * fresh process, because as_context_ensure_os_config_loaded() caches its
 * result on the #AsContext the component owns and only reloads when that
 * context has never loaded the configuration before.
 */

#include <appstream.h>

#include <stdio.h>

int
main (int argc, char **argv)
{
	AsComponent *cpt;
	gboolean floss;
	gchar *pkgnames[2];

	if (argc != 2) {
		fprintf (stderr, "usage: %s ORIGIN\n", argv[0]);
		return 2;
	}

	pkgnames[0] = (gchar *) "nex-uapi-smoke-pkg";
	pkgnames[1] = NULL;

	cpt = as_component_new ();
	as_component_set_pkgnames (cpt, pkgnames);
	as_component_set_origin (cpt, argv[1]);

	floss = as_component_is_floss (cpt);
	printf ("floss: %s\n", floss ? "yes" : "no");

	g_object_unref (cpt);
	return 0;
}
