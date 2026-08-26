/*
 * Exercise the real, newly built libvulkan.so through vkCreateInstance().
 *
 * vkCreateInstance() calls loader_scan_for_layers(), loader_icd_scan(), and
 * get_loader_settings() before it can determine whether a usable driver was
 * found, so it always runs the default driver manifest search, the default
 * implicit-layer manifest search, and the vk_loader_settings.json search,
 * whether or not a working ICD is actually present. With VK_LOADER_DEBUG=all
 * set, read_data_files_in_search_paths() logs the absolute path of every
 * manifest file its scan selected, and get_loader_settings() logs the
 * absolute path of the settings file it selected. This smoke never needs a
 * working ICD: it only inspects that log, produced by the real reader, to
 * prove which planted file each case selected. A missing driver makes
 * vkCreateInstance return VK_ERROR_INCOMPATIBLE_DRIVER, which this program
 * reports but does not treat as failure.
 */

#include <vulkan/vulkan.h>

#include <stdio.h>

int main(void) {
  VkApplicationInfo app_info = {0};
  app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
  app_info.pApplicationName = "vulkan-loader-uapi-smoke";
  app_info.apiVersion = VK_API_VERSION_1_0;

  VkInstanceCreateInfo create_info = {0};
  create_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
  create_info.pApplicationInfo = &app_info;

  VkInstance instance = VK_NULL_HANDLE;
  VkResult result = vkCreateInstance(&create_info, NULL, &instance);
  if (instance != VK_NULL_HANDLE) {
    vkDestroyInstance(instance, NULL);
  }
  fprintf(stderr, "vulkan-loader-uapi-smoke: vkCreateInstance returned %d\n", (int)result);
  return 0;
}
