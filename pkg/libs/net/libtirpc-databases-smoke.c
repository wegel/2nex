#include <netconfig.h>
#include <rpc/rpc.h>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

static const char *const test_entries[] = {
	"vendor-net",
	"runtime-net",
	"admin-net",
};

static void
check_direct_entries(const char *expected)
{
	size_t i;

	for (i = 0; i < sizeof(test_entries) / sizeof(test_entries[0]); i++) {
		struct netconfig *entry = getnetconfigent(test_entries[i]);
		int should_exist = strcmp(expected, test_entries[i]) == 0;

		if ((entry != NULL) != should_exist) {
			fprintf(stderr, "netconfig entry %s was unexpectedly %s\n",
				test_entries[i], should_exist ? "absent" : "present");
			exit(EXIT_FAILURE);
		}
		freenetconfigent(entry);
	}
}

static void
check_session_entries(const char *expected)
{
	int found[3] = { 0, 0, 0 };
	struct netconfig *entry;
	void *handle;
	size_t i;

	handle = setnetconfig();
	if (handle == NULL) {
		fprintf(stderr, "could not open the netconfig database\n");
		exit(EXIT_FAILURE);
	}
	while ((entry = getnetconfig(handle)) != NULL) {
		for (i = 0; i < sizeof(test_entries) / sizeof(test_entries[0]); i++)
			if (strcmp(entry->nc_netid, test_entries[i]) == 0)
				found[i] = 1;
	}
	if (endnetconfig(handle) != 0) {
		fprintf(stderr, "could not close the netconfig session\n");
		exit(EXIT_FAILURE);
	}

	for (i = 0; i < sizeof(test_entries) / sizeof(test_entries[0]); i++) {
		int should_exist = strcmp(expected, test_entries[i]) == 0;

		if (found[i] != should_exist) {
			fprintf(stderr, "session entry %s was unexpectedly %s\n",
				test_entries[i], should_exist ? "absent" : "present");
			exit(EXIT_FAILURE);
		}
	}
}

static void
check_reserved_port(int should_be_blocked)
{
	struct sockaddr_in address = {
		.sin_family = AF_INET,
		.sin_port = htons(700),
		.sin_addr.s_addr = htonl(INADDR_ANY),
	};
	socklen_t length = sizeof(address);
	int socket_fd;
	unsigned int selected_port;

	socket_fd = socket(AF_INET, SOCK_DGRAM, 0);
	if (socket_fd < 0) {
		perror("socket");
		exit(EXIT_FAILURE);
	}
	if (bindresvport_sa(socket_fd, (struct sockaddr *) &address) < 0) {
		perror("bindresvport_sa");
		close(socket_fd);
		exit(EXIT_FAILURE);
	}
	if (getsockname(socket_fd, (struct sockaddr *) &address, &length) < 0) {
		perror("getsockname");
		close(socket_fd);
		exit(EXIT_FAILURE);
	}
	selected_port = ntohs(address.sin_port);
	close(socket_fd);

	if ((selected_port != 700) != should_be_blocked) {
		fprintf(stderr, "port 700 was unexpectedly %s; selected %u\n",
			should_be_blocked ? "used" : "skipped", selected_port);
		exit(EXIT_FAILURE);
	}
}

int
main(int argc, char **argv)
{
	int should_be_blocked;

	if (argc != 3 ||
	    (strcmp(argv[2], "blocked") != 0 && strcmp(argv[2], "allowed") != 0)) {
		fprintf(stderr, "usage: %s ENTRY|none blocked|allowed\n", argv[0]);
		return EXIT_FAILURE;
	}
	should_be_blocked = strcmp(argv[2], "blocked") == 0;

	check_direct_entries(argv[1]);
	check_session_entries(argv[1]);
	check_reserved_port(should_be_blocked);
	return EXIT_SUCCESS;
}
