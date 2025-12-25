# explicit disables for production kernel
CONFIG_DEBUG_KERNEL=n
# CONFIG_EXPERT is not set
# CONFIG_SYSFS_DEPRECATED is not set
# CONFIG_FW_LOADER_USER_HELPER is not set
# CONFIG_AUDIT is not set
# CONFIG_RT_GROUP_SCHED is not set
CONFIG_UEVENT_HELPER=n
CONFIG_ENCRYPTED_KEYS=n
CONFIG_TRUSTED_KEYS=n
# CONFIG_MEDIA_SUPPORT_FILTER is not set

# RestrictFileSystems= disabled (requires DEBUG_INFO_BTF + BPF_LSM)
# CONFIG_BPF_LSM is not set
# CONFIG_DEBUG_INFO_BTF is not set
