/*
 * Exercise va_parseConfig() through the built libva.so.
 *
 * va_parseConfig() carries DLL_HIDDEN visibility, so this smoke cannot link
 * against it directly; it goes through vaInitialize(), the public entry
 * point that calls va_TraceInit() and the messaging setup that reads
 * LIBVA_MESSAGING_LEVEL via va_parseConfig(). vaInitialize() keeps the
 * library's own default log callbacks installed by va_newDisplayContext(),
 * so the resulting stderr output is gated by the library's own
 * default_log_level exactly as a real caller would see it; a custom
 * callback would bypass that gate and stop proving anything about
 * va_parseConfig()'s result. A stub vaGetDriverNames callback keeps
 * vaInitialize() from touching a real GPU driver while still reaching the
 * va_parseConfig() call and the two log calls whose visibility depends on
 * its result.
 *
 * Usage: libva-config-smoke EXPECTED_LEVEL
 *   EXPECTED_LEVEL is the LIBVA_MESSAGING_LEVEL (0, 1, or 2) that the
 *   administrator/runtime/vendor/environment lookup should have produced.
 */

#include <va/va.h>
#include <va/va_backend.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/*
 * Exported by libva.so but declared only in the private va_internal.h, so
 * this smoke restates the prototypes instead of installing that header.
 */
extern VADisplayContextP va_newDisplayContext(void);
extern VADriverContextP va_newDriverContext(VADisplayContextP dctx);

static VAStatus
fake_vaGetDriverNames(VADisplayContextP ctx, char **drivers, unsigned int *num_drivers)
{
	(void)ctx;
	(void)drivers;
	*num_drivers = 0;
	return VA_STATUS_ERROR_OPERATION_FAILED;
}

int
main(int argc, char **argv)
{
	VADisplayContextP dctx;
	int major, minor;
	int expected_level;
	int effective_level;
	char captured[8192];
	FILE *capture;
	size_t got;
	int have_info, have_error;
	int saved_stderr;

	if (argc != 2) {
		fprintf(stderr, "usage: %s EXPECTED_LEVEL\n", argv[0]);
		return 2;
	}
	expected_level = atoi(argv[1]);

	dctx = va_newDisplayContext();
	if (dctx == NULL) {
		fprintf(stderr, "could not create a VADisplayContext\n");
		return 2;
	}
	if (va_newDriverContext(dctx) == NULL) {
		fprintf(stderr, "could not create a VADriverContext\n");
		return 2;
	}
	dctx->vaGetDriverNames = fake_vaGetDriverNames;

	capture = tmpfile();
	if (capture == NULL) {
		perror("tmpfile");
		return 2;
	}
	fflush(stderr);
	saved_stderr = dup(fileno(stderr));
	if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
		perror("dup2");
		return 2;
	}

	vaInitialize(dctx, &major, &minor);

	fflush(stderr);
	dup2(saved_stderr, fileno(stderr));
	close(saved_stderr);
	rewind(capture);
	got = fread(captured, 1, sizeof(captured) - 1, capture);
	captured[got] = '\0';
	fclose(capture);

	have_info = strstr(captured, "VA-API version") != NULL;
	have_error = strstr(captured, "vaGetDriverNames() failed") != NULL;
	effective_level = have_info ? 2 : (have_error ? 1 : 0);

	if (effective_level != expected_level) {
		fprintf(stderr, "LIBVA_MESSAGING_LEVEL resolved to %d, expected %d\n",
			effective_level, expected_level);
		return 1;
	}
	return 0;
}
