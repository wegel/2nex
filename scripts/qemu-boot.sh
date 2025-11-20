#!/bin/sh
set -eu

qemu-system-x86_64 \
  -enable-kvm \
  -machine type=q35,accel=kvm \
  -cpu host,-hypervisor \
  -smp 4 \
  -m 4G \
  -kernel tmp/kernel.efi \
  -append "root=/dev/vda1 rw console=ttyS0,115200 earlycon=uart8250,io,0x3f8,115200" \
  -device virtio-net-pci,netdev=net0 \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-rng-pci \
  -serial mon:stdio \
  -display none
