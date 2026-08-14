#include <security/pam_appl.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
	int result;
	int expected;

	if (argc != 2 || (strcmp(argv[1], "allow") != 0 && strcmp(argv[1], "deny") != 0)) {
		fprintf(stderr, "usage: %s allow|deny\n", argv[0]);
		return EXIT_FAILURE;
	}

	expected = strcmp(argv[1], "allow") == 0 ? PAM_SUCCESS : PAM_AUTH_ERR;
	result = pam_start("uapi-service-smoke", "root", &conv, &handle);
	if (result == PAM_SUCCESS)
		result = pam_authenticate(handle, 0);
	if (handle != NULL)
		pam_end(handle, result);

	if (result != expected) {
		fprintf(stderr, "expected PAM result %d, got %d\n", expected, result);
		return EXIT_FAILURE;
	}

	return EXIT_SUCCESS;
}
