# gates.frag - bool subsystems that unlock driver availability
# applied BEFORE allmod to ensure dependent drivers get enabled

# virtualization guest support (unlocks virtio, paravirt drivers)
CONFIG_HYPERVISOR_GUEST=y
CONFIG_PARAVIRT=y
CONFIG_KVM_GUEST=y
CONFIG_XEN=y

# staging drivers (broader hardware support, some rough edges)
CONFIG_STAGING=y

# expert mode (unlocks some hardware options)
CONFIG_EXPERT=y

# wireless subsystem (unlocks wifi drivers)
CONFIG_WIRELESS=y

# media subsystem (unlocks webcams, TV tuners, etc.)
CONFIG_MEDIA_SUPPORT=y
CONFIG_MEDIA_SUPPORT_FILTER=n

# input misc devices
CONFIG_INPUT_MISC=y

# platform drivers (laptop-specific hardware)
CONFIG_X86_PLATFORM_DEVICES=y

# Intel Low Power Subsystem (modern Intel laptops)
CONFIG_X86_INTEL_LPSS=y

# AMD platform devices
CONFIG_X86_AMD_PLATFORM_DEVICE=y

# x2APIC (modern interrupt handling)
CONFIG_X86_X2APIC=y

# Intel CPU idle driver (power management)
CONFIG_INTEL_IDLE=y

# firmware loading
CONFIG_FW_LOADER=y

# GPIO (needed by some platform drivers)
CONFIG_GPIOLIB=y

# LED subsystem
CONFIG_NEW_LEDS=y
CONFIG_LEDS_CLASS=y

# power supply/battery
CONFIG_POWER_SUPPLY=y

# hwmon (hardware monitoring)
CONFIG_HWMON=y

# thermal
CONFIG_THERMAL=y

# backlight
CONFIG_BACKLIGHT_CLASS_DEVICE=y

# watchdog
CONFIG_WATCHDOG=y

# DMA engine
CONFIG_DMADEVICES=y

# IIO (sensors)
CONFIG_IIO=y

# PWM (pulse width modulation - fans, backlights, LEDs)
CONFIG_PWM=y

# Chrome/Chromebook platform drivers
CONFIG_CHROME_PLATFORMS=y

# Microsoft Surface platform drivers
CONFIG_SURFACE_PLATFORMS=y

# reliability/availability/serviceability
CONFIG_RAS=y

# accessibility devices
CONFIG_ACCESSIBILITY=y

# memory subsystem drivers
CONFIG_MEMORY=y

# interconnect bus drivers
CONFIG_INTERCONNECT=y

# android binder (container support)
CONFIG_ANDROID=y

# management component transport protocol
CONFIG_MCTP=y

# NFC subsystem
CONFIG_NFC=y

# NVMEM (non-volatile memory - eFuses, OTP)
CONFIG_NVMEM=y

# mailbox (inter-processor communication)
CONFIG_MAILBOX=y

# remote processor framework
CONFIG_REMOTEPROC=y

# remote processor messaging
CONFIG_RPMSG=y

# FPGA framework
CONFIG_FPGA=y

# auxiliary bus
CONFIG_AUXILIARY_BUS=y

# vDPA (virtio data path acceleration)
CONFIG_VDPA=y

# vhost (in-kernel virtio backend)
CONFIG_VHOST=y
CONFIG_VHOST_MENU=y

# MHI bus (Qualcomm modem host interface)
CONFIG_MHI_BUS=y

# MOST bus (automotive networking)
CONFIG_MOST=y

# Greybus (modular phone hardware)
CONFIG_GREYBUS=y

# SIOX (Eckelmann industrial I/O)
CONFIG_SIOX=y

# HSI (high-speed synchronous serial)
CONFIG_HSI=y

# W1 (1-wire bus)
CONFIG_W1=y

# SPMI (system power management interface)
CONFIG_SPMI=y

# I3C (improved I2C)
CONFIG_I3C=y

# Slimbus (audio/power delivery)
CONFIG_SLIMBUS=y

# Soundwire (audio)
CONFIG_SOUNDWIRE=y

# GNSS (GPS receivers)
CONFIG_GNSS=y

# counter subsystem
CONFIG_COUNTER=y

# DAX (direct access for persistent memory)
CONFIG_DAX=y

# CXL (compute express link)
CONFIG_CXL_BUS=y

# PCI hotplug
CONFIG_HOTPLUG_PCI=y

# PCMCIA/Cardbus
CONFIG_PCCARD=y
CONFIG_PCMCIA=y

# parallel port
CONFIG_PARPORT=y

# comedi (data acquisition)
CONFIG_COMEDI=y

# InfiniBand/RDMA
CONFIG_INFINIBAND=y

# VME bus
CONFIG_VME_BUS=y

# industrial I/O
CONFIG_IPACK_BUS=y

# mcb (MEN chameleon bus)
CONFIG_MCB=y

# rapid I/O
CONFIG_RAPIDIO=y

# EDAC (error detection and correction)
CONFIG_EDAC=y

# devfreq (dynamic frequency scaling for devices)
CONFIG_PM_DEVFREQ=y

# generic PHY framework
CONFIG_GENERIC_PHY=y

# regulator framework
CONFIG_REGULATOR=y

# reset controller
CONFIG_RESET_CONTROLLER=y

# extcon (external connector)
CONFIG_EXTCON=y

# memory technology devices (flash, etc)
CONFIG_MTD=y

# UIO (userspace I/O)
CONFIG_UIO=y

# VFIO (userspace device access for VMs)
CONFIG_VFIO=y

# power capping
CONFIG_POWERCAP=y

# ACPI error handling (modern systems)
CONFIG_ACPI_APEI=y

# NUMA (multi-socket systems)
CONFIG_NUMA=y

# PCI IOV (SR-IOV virtualization)
CONFIG_PCI_IOV=y
