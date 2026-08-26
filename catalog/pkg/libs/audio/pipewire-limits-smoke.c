#include <security/pam_appl.h>

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
	struct rlimit before;
	struct rlimit after;
	rlim_t expected_limit;
	int expected_result;
	int result;
	int status = EXIT_SUCCESS;

	if (argc != 3 ||
	    (strcmp(argv[1], "success") != 0 && strcmp(argv[1], "denied") != 0)) {
		fprintf(stderr, "usage: %s success|denied BYTES|unchanged\n", argv[0]);
		return EXIT_FAILURE;
	}

	expected_result = strcmp(argv[1], "success") == 0 ?
		PAM_SUCCESS : PAM_PERM_DENIED;
	if (getrlimit(RLIMIT_MEMLOCK, &before) != 0) {
		perror("getrlimit");
		return EXIT_FAILURE;
	}
	if (strcmp(argv[2], "unchanged") == 0) {
		expected_limit = before.rlim_cur;
	} else {
		char *end = NULL;
		expected_limit = (rlim_t) strtoull(argv[2], &end, 10);
		if (end == argv[2] || *end != '\0') {
			fprintf(stderr, "invalid byte limit: %s\n", argv[2]);
			return EXIT_FAILURE;
		}
	}

	result = pam_start("pipewire-limits-smoke", "pipewire", &conv, &handle);
	if (result == PAM_SUCCESS)
		result = pam_open_session(handle, 0);
	if (getrlimit(RLIMIT_MEMLOCK, &after) != 0) {
		perror("getrlimit");
		status = EXIT_FAILURE;
	} else if (after.rlim_cur != expected_limit) {
		fprintf(stderr, "expected memlock %llu, got %llu\n",
			(unsigned long long) expected_limit,
			(unsigned long long) after.rlim_cur);
		status = EXIT_FAILURE;
	}

	if (result != expected_result) {
		fprintf(stderr, "expected PAM result %d, got %d\n",
			expected_result, result);
		status = EXIT_FAILURE;
	}
	if (handle != NULL)
		pam_end(handle, result);

	return status;
}
