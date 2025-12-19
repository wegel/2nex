# builtins.frag - options that MUST be =y (built-in) for boot
# philosophy: boot on 95% of desktops + external drives just work

# =============================================================================
# STORAGE CONTROLLERS
# =============================================================================

# SATA/IDE
CONFIG_ATA=y
CONFIG_SATA_AHCI=y
CONFIG_SATA_AHCI_PLATFORM=y
CONFIG_ATA_PIIX=y
CONFIG_PATA_AMD=y
CONFIG_PATA_OLDPIIX=y

# NVMe
CONFIG_BLK_DEV_NVME=y
CONFIG_NVME_CORE=y

# SCSI layer (disk abstraction)
CONFIG_SCSI=y
CONFIG_BLK_DEV_SD=y

# USB storage
CONFIG_USB=y
CONFIG_USB_SUPPORT=y
CONFIG_USB_XHCI_HCD=y
CONFIG_USB_XHCI_PCI=y
CONFIG_USB_EHCI_HCD=y
CONFIG_USB_EHCI_PCI=y
CONFIG_USB_OHCI_HCD=y
CONFIG_USB_OHCI_PCI=y
CONFIG_USB_STORAGE=y

# USB4/Thunderbolt (modern docks and external storage)
CONFIG_USB4=y

# VM storage
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_BLK=y
CONFIG_VIRTIO_CONSOLE=y

# SD/MMC
CONFIG_MMC=y
CONFIG_MMC_BLOCK=y
CONFIG_MMC_SDHCI=y
CONFIG_MMC_SDHCI_PCI=y
CONFIG_MMC_SDHCI_ACPI=y

# device mapper (LVM, LUKS)
CONFIG_BLK_DEV_DM=y
CONFIG_DM_CRYPT=y
CONFIG_DM_MIRROR=y
CONFIG_DM_SNAPSHOT=y
CONFIG_BLK_DEV_LOOP=y

# =============================================================================
# FILESYSTEMS - all common ones built-in for external drives
# =============================================================================

# Linux native
CONFIG_EXT2_FS=y
CONFIG_EXT3_FS=y
CONFIG_EXT4_FS=y
CONFIG_BTRFS_FS=y
CONFIG_XFS_FS=y
CONFIG_F2FS_FS=y

# Windows/cross-platform
CONFIG_FAT_FS=y
CONFIG_VFAT_FS=y
CONFIG_EXFAT_FS=y
CONFIG_NTFS3_FS=y

# optical media
CONFIG_ISO9660_FS=y
CONFIG_UDF_FS=y

# special
CONFIG_TMPFS=y
CONFIG_TMPFS_POSIX_ACL=y
CONFIG_PROC_FS=y
CONFIG_SYSFS=y
CONFIG_FUSE_FS=y
CONFIG_OVERLAY_FS=y

# NLS for FAT/NTFS
CONFIG_NLS_CODEPAGE_437=y
CONFIG_NLS_ISO8859_1=y
CONFIG_NLS_UTF8=y

# =============================================================================
# BOOT INFRASTRUCTURE
# =============================================================================

# PCI
CONFIG_PCI=y
CONFIG_PCI_MSI=y

# ACPI
CONFIG_ACPI=y
CONFIG_ACPI_BUTTON=y
CONFIG_ACPI_FAN=y
CONFIG_ACPI_PROCESSOR=y
CONFIG_ACPI_THERMAL=y

# device creation
CONFIG_DEVTMPFS=y
CONFIG_DEVTMPFS_MOUNT=y

# binary formats
CONFIG_BINFMT_ELF=y
CONFIG_BINFMT_SCRIPT=y

# EFI
CONFIG_EFI=y
CONFIG_EFI_STUB=y
CONFIG_EFI_MIXED=y
CONFIG_EFIVAR_FS=y

# initramfs decompression
CONFIG_BLK_DEV_INITRD=y
CONFIG_RD_GZIP=y
CONFIG_RD_BZIP2=y
CONFIG_RD_LZMA=y
CONFIG_RD_XZ=y
CONFIG_RD_LZO=y
CONFIG_RD_LZ4=y
CONFIG_RD_ZSTD=y

# =============================================================================
# CONSOLE/INPUT - essential UX
# =============================================================================

# console output
CONFIG_TTY=y
CONFIG_VT=y
CONFIG_VT_CONSOLE=y
CONFIG_UNIX98_PTYS=y
CONFIG_FRAMEBUFFER_CONSOLE=y
CONFIG_VGA_CONSOLE=y
CONFIG_DUMMY_CONSOLE=y
CONFIG_FB=y
CONFIG_FB_EFI=y
CONFIG_PRINTK=y

# serial console
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y
CONFIG_SERIAL_8250_PCI=y

# early graphics
CONFIG_DRM_SIMPLEDRM=y

# keyboard
CONFIG_INPUT=y
CONFIG_INPUT_KEYBOARD=y
CONFIG_KEYBOARD_ATKBD=y
CONFIG_HID=y
CONFIG_HID_GENERIC=y
CONFIG_USB_HID=y

# =============================================================================
# CRYPTO (for LUKS, wifi, etc.)
# =============================================================================

CONFIG_CRYPTO=y
CONFIG_CRYPTO_XTS=y
CONFIG_CRYPTO_ESSIV=y
CONFIG_CRYPTO_AES_NI_INTEL=y
CONFIG_CRYPTO_SHA256_SSSE3=y
CONFIG_CRYPTO_SHA512_SSSE3=y
CONFIG_CRYPTO_USER_API_HASH=y
CONFIG_CRYPTO_USER_API_SKCIPHER=y
CONFIG_KEY_DH_OPERATIONS=y

# =============================================================================
# NETWORKING CORE (drivers as modules)
# =============================================================================

CONFIG_NET=y
CONFIG_INET=y
CONFIG_IPV6=y
CONFIG_NETDEVICES=y

# =============================================================================
# MODULES
# =============================================================================

CONFIG_MODULES=y
CONFIG_MODULE_UNLOAD=y
CONFIG_MODULE_COMPRESS_ZSTD=y
