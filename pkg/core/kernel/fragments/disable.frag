# disable.frag - kill debug, testing, and security blockers
# applied AFTER allmod to disable unwanted options

# =============================================================================
# DEBUG INFRASTRUCTURE
# =============================================================================

# CONFIG_DEBUG_KERNEL is not set
CONFIG_DEBUG_INFO_NONE=y
# CONFIG_DEBUG_INFO_DWARF_TOOLCHAIN_DEFAULT is not set
# CONFIG_DEBUG_INFO_DWARF4 is not set
# CONFIG_DEBUG_INFO_DWARF5 is not set
# CONFIG_DEBUG_MISC is not set
# CONFIG_DEBUG_FS is not set
# CONFIG_FRAME_POINTER is not set

# sanitizers
# CONFIG_KASAN is not set
# CONFIG_KCSAN is not set
# CONFIG_KMSAN is not set
# CONFIG_UBSAN is not set

# tracing/profiling
# CONFIG_KPROBES is not set
# CONFIG_FTRACE is not set
# CONFIG_TRACING is not set
# CONFIG_PROFILING is not set
# CONFIG_GCOV_KERNEL is not set
# CONFIG_KCOV is not set

# lock debugging
# CONFIG_LOCKDEP is not set
# CONFIG_PROVE_LOCKING is not set
# CONFIG_LOCK_STAT is not set

# memory debugging
# CONFIG_KFENCE is not set
# CONFIG_DEBUG_KMEMLEAK is not set
# CONFIG_SLUB_DEBUG is not set
# CONFIG_SLUB_DEBUG_ON is not set

# =============================================================================
# TESTING FRAMEWORKS
# =============================================================================

# CONFIG_KUNIT is not set
# CONFIG_COMPILE_TEST is not set
# CONFIG_FAULT_INJECTION is not set
# CONFIG_RUNTIME_TESTING_MENU is not set

# self-tests (comprehensive list)
# CONFIG_OF_UNITTEST is not set
# CONFIG_BTRFS_FS_RUN_SANITY_TESTS is not set
# CONFIG_ASYNC_RAID6_TEST is not set
# CONFIG_ATOMIC64_SELFTEST is not set
# CONFIG_CRC32_SELFTEST is not set
# CONFIG_CRYPTO_TEST is not set
# CONFIG_DMABUF_SELFTESTS is not set
# CONFIG_DMAPOOL_TEST is not set
# CONFIG_EFI_TEST is not set
# CONFIG_GLOB_SELFTEST is not set
# CONFIG_GUP_TEST is not set
# CONFIG_KALLSYMS_SELFTEST is not set
# CONFIG_MMC_TEST is not set
# CONFIG_NET_SELFTESTS is not set
# CONFIG_RANDOM32_SELFTEST is not set
# CONFIG_REED_SOLOMON_TEST is not set
# CONFIG_RTC_DRV_TEST is not set
# CONFIG_STATIC_CALL_SELFTEST is not set
# CONFIG_STATIC_KEYS_SELFTEST is not set
# CONFIG_STMMAC_SELFTESTS is not set
# CONFIG_USB_TEST is not set
# CONFIG_WW_MUTEX_SELFTEST is not set
# CONFIG_XZ_DEC_TEST is not set

# test modules
# CONFIG_TEST_KMOD is not set
# CONFIG_TEST_FIRMWARE is not set
# CONFIG_TEST_SYSCTL is not set

# RCU testing
# CONFIG_RCU_SCALE_TEST is not set
# CONFIG_RCU_TORTURE_TEST is not set
# CONFIG_RCU_REF_SCALE_TEST is not set
# CONFIG_RCU_TRACE is not set
# CONFIG_RCU_EQS_DEBUG is not set

# =============================================================================
# SAMPLES
# =============================================================================

# CONFIG_SAMPLES is not set

# =============================================================================
# SECURITY BLOCKERS (IMA/EVM block unsigned modules)
# =============================================================================

# CONFIG_IMA is not set
# CONFIG_IMA_APPRAISE is not set
# CONFIG_EVM is not set
# CONFIG_INTEGRITY is not set
# CONFIG_AUDIT is not set

# LSMs that can interfere with module loading
# CONFIG_SECURITY is not set
# CONFIG_SECURITY_SELINUX is not set
# CONFIG_SECURITY_APPARMOR is not set
# CONFIG_SECURITY_TOMOYO is not set
# CONFIG_SECURITY_SMACK is not set
# CONFIG_SECURITY_LANDLOCK is not set
# CONFIG_SECURITY_YAMA is not set
# CONFIG_SECURITY_SAFESETID is not set
# CONFIG_SECURITY_LOADPIN is not set
# CONFIG_SECURITY_LOCKDOWN_LSM is not set
# CONFIG_SECURITY_IPE is not set

# module signing (would reject unsigned modules)
# CONFIG_MODULE_SIG is not set
# CONFIG_SYSTEM_TRUSTED_KEYRING is not set
# CONFIG_SECONDARY_TRUSTED_KEYRING is not set
# CONFIG_SYSTEM_BLACKLIST_KEYRING is not set

# =============================================================================
# MODULE COMPATIBILITY
# =============================================================================

# MODVERSIONS causes "Unknown symbol" with CRC mismatch
# CONFIG_MODVERSIONS is not set

# don't force-load modules with wrong version
# CONFIG_MODULE_FORCE_LOAD is not set
# CONFIG_MODULE_ALLOW_MISSING_NAMESPACE_IMPORTS is not set

# don't trim unused symbols (breaks modular kvm, etc.)
# CONFIG_TRIM_UNUSED_KSYMS is not set

# =============================================================================
# BUILD ISSUES
# =============================================================================

# -Werror causes build failures with newer GCC
# CONFIG_WERROR is not set

# GCC plugins can crash compiler
# CONFIG_GCC_PLUGINS is not set

# structure randomization breaks ABI
CONFIG_RANDSTRUCT_NONE=y

# =============================================================================
# MISC CRUFT
# =============================================================================

# staging media (broken/incomplete drivers)
# CONFIG_STAGING_MEDIA is not set

# kgdb
# CONFIG_KGDB is not set

# misc debug options
# CONFIG_MAGIC_SYSRQ is not set
# CONFIG_DETECT_HUNG_TASK is not set
# CONFIG_WQ_WATCHDOG is not set
# CONFIG_PANIC_ON_OOPS is not set
# CONFIG_KALLSYMS is not set
# CONFIG_PRINTK_CALLER is not set
# CONFIG_DYNAMIC_DEBUG is not set
# CONFIG_DYNAMIC_DEBUG_CORE is not set
