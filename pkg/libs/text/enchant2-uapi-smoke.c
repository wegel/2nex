/*
 * Exercises the real, newly built libenchant-2 UAPI-config reader this
 * package patches, through its normal broker entry points rather than a
 * reimplemented parser.
 *
 * argv[1] is a language tag that must not contain '-', '_', '@', or '.',
 * e.g. "nexuapishared": enchant_broker_set_ordering() (lib/broker.c),
 * which stores each "tag:ordering" line this package's tests write, runs
 * every tag through normalize_dictionary_tag() (lib/broker.c) before using
 * it as a hash key, and that function is a no-op on a tag built only from
 * lowercase letters and digits, so this smoke can use argv[1] as-is
 * without duplicating that normalization itself. Enchant's own build
 * restricts the shared library's exported symbols to the "enchant_*"
 * prefix (see the "^enchant_.*" filter in lib/Makefile.am's libtool
 * link rule), so normalize_dictionary_tag() itself is not callable from
 * outside the library in the first place.
 *
 * enchant_broker_init() (lib/broker.c) calls enchant_broker_load_providers()
 * to load every EnchantProvider module from the fixed /usr/lib/enchant-2
 * module directory, then calls enchant_broker_load_provider_ordering()
 * (the function this package's patch changes), which calls
 * enchant_get_conf_dirs() (lib/provider.c, also patched) and loads every
 * enchant.ordering file it finds, in that list's order, into a hash table
 * keyed by normalized language tag; a later directory's entry for the
 * same tag overwrites an earlier directory's entry via
 * g_hash_table_insert().
 *
 * enchant_broker_get_ordered_providers() (lib/broker.c) is not declared in
 * the installed enchant.h, but it is the real, exported function the
 * broker itself uses (see enchant_broker_request_dict() and
 * enchant_broker_list_dicts() in lib/broker.c) to turn a tag's merged
 * "name,name,..." ordering string into the matching loaded
 * EnchantProvider* list, in that string's order. This smoke declares its
 * prototype and calls it directly so the printed order reflects exactly
 * what the broker resolved, not an assumption about it.
 */

#include <enchant-provider.h>

#include <glib.h>
#include <stdio.h>

extern GSList *enchant_broker_get_ordered_providers (EnchantBroker * broker,
						      const char *tag);

int
main (int argc, char **argv)
{
	EnchantBroker *broker;
	GSList *providers;
	GSList *it;

	if (argc != 2) {
		fprintf (stderr, "usage: %s language-tag\n", argv[0]);
		return 2;
	}

	broker = enchant_broker_init ();
	if (broker == NULL) {
		fprintf (stderr, "enchant uapi smoke: enchant_broker_init failed\n");
		return 1;
	}

	providers = enchant_broker_get_ordered_providers (broker, argv[1]);
	if (providers == NULL) {
		printf ("unresolved: %s\n", argv[1]);
	} else {
		printf ("order:");
		for (it = providers; it != NULL; it = it->next) {
			EnchantProvider *provider = (EnchantProvider *) it->data;
			printf (" %s", provider->identify (provider));
		}
		printf ("\n");
	}

	g_slist_free (providers);
	enchant_broker_free (broker);
	return 0;
}
