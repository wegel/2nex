#include <security/pam_appl.h>

#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>

static int
conversation(int message_count, const struct pam_message **messages,
             struct pam_response **responses, void *data)
{
	(void) messages;
	(void) data;

	*responses = calloc((size_t) message_count, sizeof(**responses));
	return *responses == NULL ? PAM_BUF_ERR : PAM_SUCCESS;
}

int
main(int argc, char **argv)
{
	const struct pam_conv conv = { conversation, NULL };
	pam_handle_t *handle = NULL;
	int result = PAM_SUCCESS;
	int expected = PAM_SUCCESS;
	int exit_status = EXIT_SUCCESS;

	if (argc < 4) {
		fprintf(stderr,
			"usage: %s SERVICE auth allow|deny\n"
			"       %s SERVICE session-limit LIMIT|unchanged\n"
			"       %s SERVICE session-env NAME VALUE\n"
			"       %s SERVICE session-env-absent NAME\n",
			argv[0], argv[0], argv[0], argv[0]);
		return EXIT_FAILURE;
	}

	result = pam_start(argv[1], "root", &conv, &handle);
	if (result != PAM_SUCCESS)
		goto done;

	if (strcmp(argv[2], "auth") == 0 && argc == 4) {
		if (strcmp(argv[3], "allow") == 0)
			expected = PAM_SUCCESS;
		else if (strcmp(argv[3], "deny") == 0)
			expected = PAM_AUTH_ERR;
		else
			exit_status = EXIT_FAILURE;

		if (exit_status == EXIT_SUCCESS)
			result = pam_authenticate(handle, 0);
	} else if (strcmp(argv[2], "session-limit") == 0 && argc == 4) {
		struct rlimit before;
		struct rlimit after;
		rlim_t expected_limit;
		char *end = NULL;

		if (getrlimit(RLIMIT_NOFILE, &before) != 0) {
			perror("getrlimit");
			exit_status = EXIT_FAILURE;
			goto done;
		}
		if (strcmp(argv[3], "unchanged") == 0) {
			expected_limit = before.rlim_cur;
		} else {
			errno = 0;
			expected_limit = (rlim_t) strtoull(argv[3], &end, 10);
			if (errno != 0 || end == argv[3] || *end != '\0') {
				fprintf(stderr, "invalid limit: %s\n", argv[3]);
				exit_status = EXIT_FAILURE;
				goto done;
			}
		}

		result = pam_open_session(handle, 0);
		if (result == PAM_SUCCESS && getrlimit(RLIMIT_NOFILE, &after) != 0) {
			perror("getrlimit");
			exit_status = EXIT_FAILURE;
			goto done;
		}
		if (result == PAM_SUCCESS && after.rlim_cur != expected_limit) {
			fprintf(stderr, "expected nofile limit %llu, got %llu\n",
				(unsigned long long) expected_limit,
				(unsigned long long) after.rlim_cur);
			exit_status = EXIT_FAILURE;
		}
		if (result == PAM_SUCCESS)
			pam_close_session(handle, 0);
	} else if ((strcmp(argv[2], "session-env") == 0 && argc == 5) ||
		   (strcmp(argv[2], "session-env-absent") == 0 && argc == 4)) {
		const char *value;
		const char *expected_value = argc == 5 ? argv[4] : NULL;

		result = pam_open_session(handle, 0);
		value = result == PAM_SUCCESS ? pam_getenv(handle, argv[3]) : NULL;
		if (result == PAM_SUCCESS && expected_value != NULL &&
		    (value == NULL || strcmp(value, expected_value) != 0)) {
			fprintf(stderr, "expected %s=%s, got %s\n", argv[3], expected_value,
				value == NULL ? "(null)" : value);
			exit_status = EXIT_FAILURE;
		} else if (result == PAM_SUCCESS && expected_value == NULL &&
			   value != NULL) {
			fprintf(stderr, "expected %s to be absent, got %s\n",
				argv[3], value);
			exit_status = EXIT_FAILURE;
		}
		if (result == PAM_SUCCESS)
			pam_close_session(handle, 0);
	} else {
		fprintf(stderr, "invalid action or argument count\n");
		exit_status = EXIT_FAILURE;
	}

done:
	if (result != expected) {
		fprintf(stderr, "%s %s: expected PAM result %d, got %d\n",
			argv[1], argv[2], expected, result);
		exit_status = EXIT_FAILURE;
	}
	if (handle != NULL)
		pam_end(handle, result);

	return exit_status;
}
