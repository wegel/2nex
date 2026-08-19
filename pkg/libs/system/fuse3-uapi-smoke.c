/*
 * Drives the real, newly built, newly installed fusermount3 through its
 * normal _FUSE_COMMFD startup protocol far enough to observe the two
 * decisions that util/fusermount.c's tiered fuse.conf reader controls,
 * without a functional /dev/fuse:
 *
 *   - the mount_max check (mount_fuse(), right after read_conf() and
 *     before any device or mount syscall) prints
 *     "too many FUSE filesystems mounted; mount_max=N can be set in %s"
 *     and refuses the mount when the caller is not root and the current
 *     mount count is already at or above mount_max;
 *   - the user_allow_other check (do_mount(), while parsing -o options,
 *     also before any mount syscall) prints
 *     "option %s only allowed if 'user_allow_other' is set in %s"
 *     and refuses the mount when the caller is not root, requests
 *     allow_other or allow_root, and user_allow_other is not set.
 *
 * Both checks, and read_conf() itself, run before fusermount3 ever
 * attempts the real mount(2) syscall, so this harness never needs a
 * working FUSE connection to observe them: the caller (the shell check
 * script) plants a plain regular file at /dev/fuse so open(2) succeeds,
 * and never proceeds far enough to depend on that file behaving like the
 * real device.
 *
 * fusermount3 requires _FUSE_COMMFD to name a real AF_UNIX socket (main()
 * calls fstat() and rejects anything that is not S_ISSOCK), so this
 * harness creates a socketpair(2), forks, and executes the given
 * fusermount3 binary in the child with the socket fd's number exported as
 * _FUSE_COMMFD. The parent only waits for the child and reports its exit
 * status; every assertion the check script makes is against the text
 * fusermount3 itself printed to stderr, inherited unchanged from this
 * harness's own stderr.
 *
 * This harness does not attempt a real mount and proves nothing about
 * fusermount3's behavior once past the two checks above.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

int
main(int argc, char **argv)
{
	if (argc != 4) {
		fprintf(stderr,
			"usage: %s fusermount3-path mountpoint options-or-dash\n",
			argv[0]);
		return 2;
	}

	const char *fusermount3_path = argv[1];
	const char *mountpoint = argv[2];
	const char *options = argv[3];

	int sv[2];
	if (socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == -1) {
		perror("fuse3 uapi smoke: socketpair");
		return 1;
	}

	pid_t pid = fork();
	if (pid == -1) {
		perror("fuse3 uapi smoke: fork");
		return 1;
	}

	if (pid == 0) {
		close(sv[1]);

		char commfd[16];
		snprintf(commfd, sizeof(commfd), "%d", sv[0]);
		setenv("_FUSE_COMMFD", commfd, 1);

		char *child_argv[6];
		int i = 0;
		child_argv[i++] = (char *) fusermount3_path;
		if (strcmp(options, "-") != 0) {
			child_argv[i++] = (char *) "-o";
			child_argv[i++] = (char *) options;
		}
		child_argv[i++] = (char *) mountpoint;
		child_argv[i] = NULL;

		execv(fusermount3_path, child_argv);
		perror("fuse3 uapi smoke: execv");
		_exit(127);
	}

	close(sv[0]);
	close(sv[1]);

	int status;
	if (waitpid(pid, &status, 0) == -1) {
		perror("fuse3 uapi smoke: waitpid");
		return 1;
	}

	if (WIFEXITED(status))
		printf("fuse3 uapi smoke: fusermount3 exited %d\n",
		       WEXITSTATUS(status));
	else
		printf("fuse3 uapi smoke: fusermount3 terminated by signal %d\n",
		       WTERMSIG(status));

	return 0;
}
