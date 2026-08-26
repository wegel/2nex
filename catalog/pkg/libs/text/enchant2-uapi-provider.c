/*
 * A minimal EnchantProvider module for the enchant2 UAPI check. It never
 * checks or suggests a real word; enchant_broker_get_ordered_providers()
 * (lib/broker.c) only calls each loaded provider's identify() method to
 * match it against the "tag:ordering" strings the broker read from
 * enchant.ordering files, so this module supplies just enough of the
 * EnchantProvider contract (lib/enchant-provider.h) to load and identify
 * itself, and nothing a dictionary lookup would need.
 *
 * PROVIDER_NAME must be supplied at compile time, e.g.
 * -DPROVIDER_NAME='"nex-uapi-want"'. The check script builds two copies of
 * this file under different PROVIDER_NAME values and installs both into the
 * real /usr/lib/enchant-2 module directory so enchant_broker_init() loads
 * them the same way it would load a real spelling backend.
 */

#include <enchant-provider.h>

#ifndef PROVIDER_NAME
#error "PROVIDER_NAME must be defined, e.g. -DPROVIDER_NAME=\"nex-uapi-want\""
#endif

static EnchantDict *
nex_uapi_request_dict (EnchantProvider * me, const char *const tag)
{
	(void) me;
	(void) tag;
	return NULL;
}

static void
nex_uapi_dispose_dict (EnchantProvider * me, EnchantDict * dict)
{
	(void) me;
	(void) dict;
}

static const char *
nex_uapi_identify (EnchantProvider * me)
{
	(void) me;
	return PROVIDER_NAME;
}

static const char *
nex_uapi_describe (EnchantProvider * me)
{
	(void) me;
	return PROVIDER_NAME " (Nex UAPI check provider)";
}

static char **
nex_uapi_list_dicts (EnchantProvider * me, size_t * out_n_dicts)
{
	(void) me;
	*out_n_dicts = 0;
	return NULL;
}

EnchantProvider *
init_enchant_provider (void)
{
	EnchantProvider *provider = enchant_provider_new ();

	provider->request_dict = nex_uapi_request_dict;
	provider->dispose_dict = nex_uapi_dispose_dict;
	provider->identify = nex_uapi_identify;
	provider->describe = nex_uapi_describe;
	provider->list_dicts = nex_uapi_list_dicts;

	return provider;
}
