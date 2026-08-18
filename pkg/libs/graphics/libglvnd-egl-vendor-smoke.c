#include <EGL/egl.h>
#include <EGL/eglext.h>

#include <stdio.h>
#include <stdlib.h>

int main(void) {
  PFNEGLQUERYDEVICESEXTPROC queryDevices;
  EGLDeviceEXT *devices;
  EGLint count = -1;
  EGLint i;

  /* EGL_EXT_device_enumeration entrypoints are only reachable through
   * eglGetProcAddress; libEGL does not export them directly. */
  queryDevices =
      (PFNEGLQUERYDEVICESEXTPROC)eglGetProcAddress("eglQueryDevicesEXT");
  if (queryDevices == NULL) {
    fprintf(stderr, "eglGetProcAddress(eglQueryDevicesEXT) failed\n");
    return 1;
  }

  if (!queryDevices(0, NULL, &count)) {
    fprintf(stderr, "eglQueryDevicesEXT (count) failed: 0x%04x\n",
            eglGetError());
    return 1;
  }

  printf("%d\n", count);
  if (count == 0) {
    return 0;
  }

  devices = calloc((size_t)count, sizeof(*devices));
  if (devices == NULL) {
    return 1;
  }
  if (!queryDevices(count, devices, &count)) {
    fprintf(stderr, "eglQueryDevicesEXT (devices) failed: 0x%04x\n",
            eglGetError());
    free(devices);
    return 1;
  }

  for (i = 0; i < count; i++) {
    EGLDisplay display;
    EGLint major, minor;
    const char *vendor;

    display = eglGetPlatformDisplay(EGL_PLATFORM_DEVICE_EXT, devices[i], NULL);
    if (display == EGL_NO_DISPLAY) {
      fprintf(stderr, "eglGetPlatformDisplay failed at %d: 0x%04x\n", i,
              eglGetError());
      free(devices);
      return 1;
    }
    if (!eglInitialize(display, &major, &minor)) {
      fprintf(stderr, "eglInitialize failed at %d: 0x%04x\n", i,
              eglGetError());
      free(devices);
      return 1;
    }
    vendor = eglQueryString(display, EGL_VENDOR);
    if (vendor == NULL) {
      fprintf(stderr, "eglQueryString failed at %d: 0x%04x\n", i,
              eglGetError());
      free(devices);
      return 1;
    }
    puts(vendor);
  }

  free(devices);
  return 0;
}
