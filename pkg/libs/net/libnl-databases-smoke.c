#define _GNU_SOURCE

#include <netlink/route/pktloc.h>
#include <netlink/route/tc.h>

#include <linux/pkt_sched.h>

#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const struct timespec fixed_time[2] = {
	{ .tv_sec = 1704067200, .tv_nsec = 0 },
	{ .tv_sec = 1704067200, .tv_nsec = 0 },
};
static const struct timespec changed_time[2] = {
	{ .tv_sec = 1704067201, .tv_nsec = 0 },
	{ .tv_sec = 1704067201, .tv_nsec = 0 },
};

static void
fail(const char *message)
{
	fprintf(stderr, "%s: %s\n", message, strerror(errno));
	exit(EXIT_FAILURE);
}

static void
make_directory(const char *path)
{
	if (mkdir(path, 0755) < 0 && errno != EEXIST)
		fail(path);
}

static void
write_file(const char *path, const char *contents)
{
	FILE *file = fopen(path, "we");

	if (file == NULL)
		fail(path);
	if (fputs(contents, file) == EOF || fclose(file) != 0)
		fail(path);
	if (utimensat(AT_FDCWD, path, fixed_time, 0) < 0)
		fail(path);
}

static void
remove_file(const char *path)
{
	if (unlink(path) < 0 && errno != ENOENT)
		fail(path);
}

static void
expect_class(const char *name, int present)
{
	uint32_t handle;
	int result = rtnl_tc_str2handle(name, &handle);

	if ((present && result < 0) || (!present && result == 0)) {
		fprintf(stderr, "class %s was unexpectedly %s\n", name,
			present ? "absent" : "present");
		exit(EXIT_FAILURE);
	}
}

static void
expect_location(const char *name, int present)
{
	struct rtnl_pktloc *location = NULL;
	int result = rtnl_pktloc_lookup(name, &location);

	if ((present && result < 0) || (!present && result == 0)) {
		fprintf(stderr, "packet location %s was unexpectedly %s\n", name,
			present ? "absent" : "present");
		if (location != NULL)
			rtnl_pktloc_put(location);
		exit(EXIT_FAILURE);
	}
	if (location != NULL)
		rtnl_pktloc_put(location);
}

static void
reload_classes(void)
{
	int result = rtnl_tc_read_classid_file();

	if (result < 0) {
		fprintf(stderr, "could not reload class database: %d\n", result);
		exit(EXIT_FAILURE);
	}
}

static void
expect_file_contains(const char *path, const char *needle)
{
	char buffer[8192];
	FILE *file = fopen(path, "re");
	size_t length;

	if (file == NULL)
		fail(path);
	length = fread(buffer, 1, sizeof(buffer) - 1, file);
	if (ferror(file) || fclose(file) != 0)
		fail(path);
	buffer[length] = '\0';
	if (strstr(buffer, needle) == NULL) {
		fprintf(stderr, "%s does not contain %s\n", path, needle);
		exit(EXIT_FAILURE);
	}
}

int
main(int argc, char **argv)
{
	char explicit_classid[4096];
	char explicit_pktloc[4096];
	uint32_t generated;

	if (argc != 2) {
		fprintf(stderr, "usage: %s EXPLICIT_DIRECTORY\n", argv[0]);
		return EXIT_FAILURE;
	}
	if (snprintf(explicit_classid, sizeof(explicit_classid), "%s/classid",
		     argv[1]) >= (int) sizeof(explicit_classid) ||
	    snprintf(explicit_pktloc, sizeof(explicit_pktloc), "%s/pktloc",
		     argv[1]) >= (int) sizeof(explicit_pktloc)) {
		fprintf(stderr, "explicit directory path is too long\n");
		return EXIT_FAILURE;
	}

	expect_class("vendor-class", 1);
	expect_location("ip.src", 1);

	make_directory("/run/libnl");
	write_file("/run/libnl/classid", "2:2 runtime-class\n");
	write_file("/run/libnl/pktloc", "runtime.loc u8 net+1\n");
	reload_classes();
	expect_class("runtime-class", 1);
	expect_class("vendor-class", 0);
	expect_location("runtime.loc", 1);
	expect_location("ip.src", 0);

	make_directory("/etc/libnl");
	write_file("/etc/libnl/classid", "3:3 admin-class\n");
	write_file("/etc/libnl/pktloc", "admin.loc u8 net+2\n");
	reload_classes();
	expect_class("admin-class", 1);
	expect_class("runtime-class", 0);
	expect_location("admin.loc", 1);
	expect_location("runtime.loc", 0);

	write_file("/etc/libnl/classid", "");
	write_file("/etc/libnl/pktloc", "");
	if (utimensat(AT_FDCWD, "/etc/libnl/classid", changed_time, 0) < 0 ||
	    utimensat(AT_FDCWD, "/etc/libnl/pktloc", changed_time, 0) < 0)
		fail("empty administrator database timestamp");
	reload_classes();
	expect_class("admin-class", 0);
	expect_class("runtime-class", 0);
	expect_location("admin.loc", 0);
	expect_location("runtime.loc", 0);

	remove_file("/etc/libnl/classid");
	remove_file("/etc/libnl/pktloc");
	reload_classes();
	expect_class("runtime-class", 1);
	expect_location("runtime.loc", 1);

	remove_file("/run/libnl/classid");
	remove_file("/run/libnl/pktloc");
	reload_classes();
	expect_class("vendor-class", 1);
	expect_location("ip.src", 1);

	make_directory(argv[1]);
	write_file(explicit_classid, "4:4 explicit-class\n");
	write_file(explicit_pktloc, "explicit.loc u8 net+3\n");
	if (setenv("NLSYSCONFDIR", argv[1], 1) < 0)
		fail("setenv");
	reload_classes();
	expect_class("explicit-class", 1);
	expect_class("vendor-class", 0);
	expect_location("explicit.loc", 1);
	expect_location("ip.src", 0);
	if (unsetenv("NLSYSCONFDIR") < 0)
		fail("unsetenv");

	write_file("/run/libnl/classid", "2:2 runtime-class\n");
	reload_classes();
	if (rtnl_classid_generate("generated-class", &generated, TC_H_ROOT) < 0) {
		fprintf(stderr, "could not generate a class ID\n");
		return EXIT_FAILURE;
	}
	expect_file_contains("/etc/libnl/classid", "runtime-class");
	expect_file_contains("/etc/libnl/classid", "generated-class");

	remove_file("/etc/libnl/classid");
	remove_file("/etc/libnl/pktloc");
	remove_file("/run/libnl/classid");
	remove_file("/run/libnl/pktloc");
	remove_file(explicit_classid);
	remove_file(explicit_pktloc);
	if (rmdir("/etc/libnl") < 0)
		fail("/etc/libnl");
	if (rmdir("/run/libnl") < 0)
		fail("/run/libnl");
	if (rmdir(argv[1]) < 0)
		fail(argv[1]);

	return EXIT_SUCCESS;
}
