/*
 * Exercises the real, newly built libpopt UAPI-config reader this package
 * patches, through its normal public entry points rather than a
 * reimplemented parser.
 *
 * argv[1] is the appName passed to poptGetContext(), which becomes
 * con->appName; poptReadDefaultConfig() (src/poptconfig.c) returns early
 * unless con->appName is set, so this smoke always supplies one. argv[2]
 * is a popt long option, e.g. "--probe", that this smoke expects to find
 * only as a popt "alias" line for that appName in one of the vendor,
 * /run, or /etc config tiers poptReadDefaultConfig() reads.
 *
 * The alias line's expansion is required to include "--value=STRING".
 * handleAlias() (src/popt.c) expands an unregistered long option it
 * recognizes as an alias into that alias's stored argv, which
 * poptGetNextOpt() then parses like ordinary arguments; this smoke
 * registers only "--value" as a real option, so the alias expansion is
 * what actually reaches it. Printing that string proves which tier's
 * alias line the real reader selected, not what this smoke assumes it
 * selected.
 *
 * When argv[2] matches no alias at all, poptGetNextOpt() returns
 * POPT_ERROR_BADOPT; this smoke reports that as "unresolved" rather than
 * as a failure, since a probe for a masked or absent tier is expected to
 * come back unresolved.
 */

#include <popt.h>

#include <stdio.h>

int
main(int argc, char **argv)
{
	poptContext con;
	const char *probeArgv[2];
	char *value = NULL;
	struct poptOption options[] = {
		{ "value", 0, POPT_ARG_STRING, &value, 0, NULL, NULL },
		POPT_TABLEEND
	};
	int rc;

	if (argc != 3) {
		fprintf(stderr, "usage: %s appName --long-option\n", argv[0]);
		return 2;
	}

	probeArgv[0] = argv[1];
	probeArgv[1] = argv[2];

	con = poptGetContext(argv[1], 2, probeArgv, options, 0);
	if (con == NULL) {
		fprintf(stderr, "popt uapi smoke: poptGetContext failed\n");
		return 1;
	}

	if (poptReadDefaultConfig(con, 1)) {
		fprintf(stderr,
			"popt uapi smoke: poptReadDefaultConfig failed\n");
		poptFreeContext(con);
		return 1;
	}

	rc = poptGetNextOpt(con);
	if (rc == POPT_ERROR_BADOPT) {
		printf("unresolved: %s\n", argv[2]);
		poptFreeContext(con);
		return 0;
	}
	if (rc != -1) {
		fprintf(stderr, "popt uapi smoke: poptGetNextOpt failed: %s\n",
			poptStrerror(rc));
		poptFreeContext(con);
		return 1;
	}

	printf("value: %s\n", value ? value : "(unset)");

	poptFreeContext(con);
	return 0;
}
