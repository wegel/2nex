# string/choice overrides
CONFIG_UEVENT_HELPER_PATH=""
CONFIG_LSM="landlock,lockdown,yama,integrity"

# wifi crypto requirements (iwd)
CONFIG_KEY_DH_OPERATIONS=y

# enable AMD Display Core for amdgpu KMS
CONFIG_DRM_AMD_DC=y

# keep exported symbols available for out-of-tree modules
# CONFIG_TRIM_UNUSED_KSYMS is not set

# cap maximum CPUs to reduce static kernel memory usage
CONFIG_NR_CPUS=128

# compress kernel image for size
# CONFIG_KERNEL_GZIP is not set
# CONFIG_KERNEL_BZIP2 is not set
# CONFIG_KERNEL_LZMA is not set
# CONFIG_KERNEL_XZ is not set
# CONFIG_KERNEL_LZO is not set
# CONFIG_KERNEL_LZ4 is not set
CONFIG_KERNEL_ZSTD=y
