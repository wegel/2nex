/*
 * Load the real proprietary GLX driver so its own application-profile
 * parser runs, exactly as a GLX client would trigger it. libGLX_nvidia.so
 * needs libnvidia-glcore.so, whose constructor performs the compiled
 * path-table walk and, under __GL_APPLICATION_PROFILE_LOG=1, prints one
 * "Parsing file ..." line per file it opens. No GPU is required: the
 * driver walks its path table and logs before it ever probes hardware.
 *
 * Usage: nvidia-application-profile-smoke LIBGLX_NVIDIA_PATH
 */

#include <dlfcn.h>
#include <stdio.h>

int
main(int argc, char **argv)
{
	if (argc != 2) {
		fprintf(stderr, "usage: %s LIBGLX_NVIDIA_PATH\n", argv[0]);
		return 2;
	}

	if (dlopen(argv[1], RTLD_NOW) == NULL) {
		fprintf(stderr, "dlopen failed: %s\n", dlerror());
		return 2;
	}

	return 0;
}
