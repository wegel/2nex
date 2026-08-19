/*
 * Exercises the real, newly built krb5 UAPI-config readers this package
 * patches, through their normal public entry points rather than a
 * reimplemented parser.
 *
 * "profile" mode calls krb5_init_context(), which reaches
 * os_get_default_config_files() in lib/krb5/os/init_os_ctx.c. With
 * KRB5_CONFIG unset that function returns the compiled
 * DEFAULT_PROFILE_PATH list this package's patch extended to
 * "/etc/krb5.conf:/run/krb5:/usr/share/krb5"; with KRB5_CONFIG set it
 * returns only that value, exactly as upstream documents. The context's
 * real merged profile is fetched with krb5_get_profile() and read back
 * with profile_get_string(), so this proves what the real profile
 * library selected, not what this smoke assumes it selected.
 *
 * "gss" mode calls the real gss_indicate_mechs(), which reaches
 * loadConfigFiles() in lib/gssapi/mechglue/g_initialize.c. That function
 * "does not check if any of these can actually be loaded" (see its
 * comment in g_initialize.c), so a mechanism entry naming a shared
 * library that does not exist still appears in the returned set; this
 * smoke only needs the config file to have been read, never the
 * library to load. Each returned OID is printed with gss_oid_to_str()
 * so the check script can grep for the exact OID this test planted.
 */

#include <gssapi/gssapi.h>
#include <krb5.h>
#include <profile.h>

#include <stdio.h>
#include <string.h>

static int
run_profile(void)
{
	krb5_context ctx;
	krb5_error_code retval;
	struct _profile_t *profile;
	char *value = NULL;

	retval = krb5_init_context(&ctx);
	if (retval) {
		fprintf(stderr, "krb5 uapi smoke: krb5_init_context failed: %ld\n",
			(long)retval);
		return 1;
	}

	retval = krb5_get_profile(ctx, &profile);
	if (retval) {
		fprintf(stderr, "krb5 uapi smoke: krb5_get_profile failed: %ld\n",
			(long)retval);
		krb5_free_context(ctx);
		return 1;
	}

	retval = profile_get_string(profile, "nex-uapi-smoke", "value", NULL,
				    NULL, &value);
	if (retval) {
		fprintf(stderr,
			"krb5 uapi smoke: profile_get_string failed: %ld\n",
			(long)retval);
		krb5_free_context(ctx);
		return 1;
	}

	printf("profile value: %s\n", value ? value : "(unset)");

	if (value)
		profile_release_string(value);
	krb5_free_context(ctx);
	return 0;
}

static int
run_gss(void)
{
	OM_uint32 major, minor;
	gss_OID_set mechs = GSS_C_NO_OID_SET;
	unsigned int i;

	major = gss_indicate_mechs(&minor, &mechs);
	if (major != GSS_S_COMPLETE) {
		fprintf(stderr,
			"krb5 uapi smoke: gss_indicate_mechs failed: major=%u minor=%u\n",
			(unsigned int)major, (unsigned int)minor);
		return 1;
	}

	for (i = 0; mechs != GSS_C_NO_OID_SET && i < mechs->count; i++) {
		gss_buffer_desc oid_str = GSS_C_EMPTY_BUFFER;

		major = gss_oid_to_str(&minor, &mechs->elements[i], &oid_str);
		if (major != GSS_S_COMPLETE)
			continue;
		printf("mech oid: %.*s\n", (int)oid_str.length,
		       (char *)oid_str.value);
		gss_release_buffer(&minor, &oid_str);
	}

	if (mechs != GSS_C_NO_OID_SET)
		gss_release_oid_set(&minor, &mechs);
	return 0;
}

int
main(int argc, char **argv)
{
	if (argc != 2) {
		fprintf(stderr, "usage: %s profile|gss\n", argv[0]);
		return 2;
	}

	if (strcmp(argv[1], "profile") == 0)
		return run_profile();
	if (strcmp(argv[1], "gss") == 0)
		return run_gss();

	fprintf(stderr, "usage: %s profile|gss\n", argv[0]);
	return 2;
}
