/*
 * Exercises the real installed libinput.so through
 * libinput_udev_assign_seat(), which calls the library's internal
 * libinput_init_quirks() unconditionally, before it ever tries to enumerate
 * a real input device (src/udev-seat.c documents that ordering: the log
 * handler is not usable during libinput_udev_create_context(), so quirks
 * init is deferred to assign_seat()). That means this smoke needs no real
 * input hardware to reach the quirks reader under test.
 *
 * The log handler is installed before assign_seat() runs and captures every
 * message the library logs. quirks_init_subsystem() always uses
 * QLOG_LIBINPUT_LOGGING through this path, which permanently suppresses its
 * per-file QLOG_NOISE debug lines regardless of log priority; only its
 * QLOG_ERROR lines (for example "<dir>: failed to find data files") reach
 * this handler. That is enough to prove whether LIBINPUT_QUIRKS_DIR replaced
 * the whole system search: with the environment variable unset and the real
 * vendor quirks directory empty or missing, the merge's required vendor
 * scan always logs that failure; with LIBINPUT_QUIRKS_DIR pointing at a
 * valid directory instead, the same missing vendor directory must produce
 * no such log line because the tiered merge, including its required vendor
 * check, must never run.
 */

#include <libinput.h>
#include <libudev.h>

#include <stdarg.h>
#include <stdio.h>

static void
log_handler(struct libinput *li, enum libinput_log_priority priority, const char *format,
	    va_list args)
{
	(void)li;
	(void)priority;
	vprintf(format, args);
}

static int
open_restricted(const char *path, int flags, void *user_data)
{
	(void)path;
	(void)flags;
	(void)user_data;
	return -1;
}

static void
close_restricted(int fd, void *user_data)
{
	(void)fd;
	(void)user_data;
}

static const struct libinput_interface interface = {
	.open_restricted = open_restricted,
	.close_restricted = close_restricted,
};

int
main(void)
{
	struct udev *udev = udev_new();
	if (!udev) {
		fprintf(stderr, "libinput uapi smoke: udev_new failed\n");
		return 1;
	}

	struct libinput *li = libinput_udev_create_context(&interface, NULL, udev);
	if (!li) {
		fprintf(stderr,
			"libinput uapi smoke: libinput_udev_create_context failed\n");
		udev_unref(udev);
		return 1;
	}

	libinput_log_set_priority(li, LIBINPUT_LOG_PRIORITY_ERROR);
	libinput_log_set_handler(li, log_handler);

	/* Succeeds even with no seat devices available; libinput_init_quirks()
	 * has already run by the time this call returns. */
	libinput_udev_assign_seat(li, "seat0");

	libinput_unref(li);
	udev_unref(udev);

	return 0;
}
