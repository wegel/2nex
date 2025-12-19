# overrides.frag - final tweaks after allmod
# applied last to set specific values

# =============================================================================
# COMPRESSION / SIZE
# =============================================================================

CONFIG_KERNEL_ZSTD=y

# =============================================================================
# PERFORMANCE
# =============================================================================

CONFIG_CC_OPTIMIZE_FOR_PERFORMANCE=y
CONFIG_PREEMPT_VOLUNTARY=y

# =============================================================================
# CRYPTO (accelerated implementations)
# =============================================================================

CONFIG_CRYPTO_AES_NI_INTEL=y
CONFIG_CRYPTO_SHA256_SSSE3=y
CONFIG_CRYPTO_SHA512_SSSE3=y
CONFIG_CRYPTO_SHA1_SSSE3=y

# =============================================================================
# CONTAINER SUPPORT
# =============================================================================

# namespaces
CONFIG_NAMESPACES=y
CONFIG_USER_NS=y
CONFIG_NET_NS=y
CONFIG_PID_NS=y
CONFIG_IPC_NS=y
CONFIG_UTS_NS=y

# cgroups
CONFIG_CGROUPS=y
CONFIG_CGROUP_FREEZER=y
CONFIG_CGROUP_DEVICE=y
CONFIG_CGROUP_CPUACCT=y
CONFIG_CGROUP_PERF=y
CONFIG_CGROUP_SCHED=y
CONFIG_CGROUP_PIDS=y
CONFIG_MEMCG=y
CONFIG_BLK_CGROUP=y
CONFIG_CGROUP_BPF=y

# BPF
CONFIG_BPF=y
CONFIG_BPF_SYSCALL=y
CONFIG_BPF_JIT=y

# seccomp
CONFIG_SECCOMP=y
CONFIG_SECCOMP_FILTER=y

# =============================================================================
# WIFI/BT CRYPTO (required for WPA)
# =============================================================================

CONFIG_CRYPTO_ECB=y
CONFIG_CRYPTO_CBC=y
CONFIG_CRYPTO_CMAC=y
CONFIG_CRYPTO_HMAC=y
CONFIG_CRYPTO_MD5=y
CONFIG_CRYPTO_SHA1=y
CONFIG_CRYPTO_SHA256=y
CONFIG_CRYPTO_SHA512=y
CONFIG_CRYPTO_AES=y
CONFIG_CRYPTO_DES=y

# =============================================================================
# KERNEL CONFIG ACCESSIBLE
# =============================================================================

CONFIG_IKCONFIG=y
CONFIG_IKCONFIG_PROC=y

# =============================================================================
# MISC
# =============================================================================

CONFIG_PRINTK_TIME=y
