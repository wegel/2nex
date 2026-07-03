#!/usr/bin/env bash
set -euo pipefail

src_dir="$1"
out_dir="$2"
version="$3"

install_file() {
  local mode="$1"
  local src="$2"
  local dest="$3"

  if [ -e "${src_dir}/${src}" ]; then
    install -Dm"${mode}" "${src_dir}/${src}" "${out_dir}${dest}"
  fi
}

install_link() {
  local target="$1"
  local dest="$2"

  mkdir -p "$(dirname "${out_dir}${dest}")"
  ln -sfn "${target}" "${out_dir}${dest}"
}

install_link_if_exists() {
  local target="$1"
  local dest="$2"

  if [ -e "${src_dir}/${target}" ]; then
    install_link "${target}" "${dest}"
  fi
}

for tool in \
  nvidia-bug-report.sh \
  nvidia-cuda-mps-control \
  nvidia-cuda-mps-server \
  nvidia-debugdump \
  nvidia-installer \
  nvidia-modprobe \
  nvidia-ngx-updater \
  nvidia-pcc \
  nvidia-persistenced \
  nvidia-powerd \
  nvidia-settings \
  nvidia-smi \
  nvidia-xconfig; do
  mode=0755
  if [ "${tool}" = "nvidia-modprobe" ]; then
    mode=4755
  fi
  install_file "${mode}" "${tool}" "/usr/bin/${tool}"
done

install_link nvidia-installer /usr/bin/nvidia-uninstall
install_file 0755 systemd/nvidia-sleep.sh /usr/bin/nvidia-sleep.sh

for page in "${src_dir}"/*.1.gz; do
  if [ -e "${page}" ]; then
    install -Dm644 "${page}" "${out_dir}/usr/share/man/man1/$(basename "${page}")"
  fi
done

for lib in "${src_dir}"/*.so*; do
  if [ ! -e "${lib}" ]; then
    continue
  fi

  case "$(basename "${lib}")" in
    libglxserver_nvidia.so.*|libvdpau_nvidia.so.*)
      continue
      ;;
  esac

  install -Dm755 "${lib}" "${out_dir}/usr/lib/$(basename "${lib}")"
done

install_link "libnvidia-ml.so.${version}" /usr/lib/libnvidia-ml.so.1
install_link libnvidia-ml.so.1 /usr/lib/libnvidia-ml.so
install_link "libcuda.so.${version}" /usr/lib/libcuda.so.1
install_link libcuda.so.1 /usr/lib/libcuda.so
install_link "libnvidia-opencl.so.${version}" /usr/lib/libnvidia-opencl.so.1
install_link "libOpenCL.so.1.0.0" /usr/lib/libOpenCL.so.1.0
install_link libOpenCL.so.1.0 /usr/lib/libOpenCL.so.1
install_link libOpenCL.so.1 /usr/lib/libOpenCL.so
install_link "libnvidia-ptxjitcompiler.so.${version}" /usr/lib/libnvidia-ptxjitcompiler.so.1
install_link libnvidia-ptxjitcompiler.so.1 /usr/lib/libnvidia-ptxjitcompiler.so
install_link "libcudadebugger.so.${version}" /usr/lib/libcudadebugger.so.1
install_link "libnvidia-nvvm.so.${version}" /usr/lib/libnvidia-nvvm.so.4
install_link libnvidia-nvvm.so.4 /usr/lib/libnvidia-nvvm.so
install_link "libGLX_nvidia.so.${version}" /usr/lib/libGLX_nvidia.so.0
install_link "libGLX_nvidia.so.${version}" /usr/lib/libGLX_indirect.so.0
install_link libOpenGL.so.0 /usr/lib/libOpenGL.so
install_link libGLESv1_CM.so.1.2.0 /usr/lib/libGLESv1_CM.so.1
install_link libGLESv1_CM.so.1 /usr/lib/libGLESv1_CM.so
install_link libGLESv2.so.2.1.0 /usr/lib/libGLESv2.so.2
install_link libGLESv2.so.2 /usr/lib/libGLESv2.so
install_link libGLX.so.0 /usr/lib/libGLX.so
install_link libGL.so.1.7.0 /usr/lib/libGL.so.1
install_link libGL.so.1 /usr/lib/libGL.so
install_link libEGL.so.1.1.0 /usr/lib/libEGL.so.1
install_link libEGL.so.1 /usr/lib/libEGL.so
install_link "libEGL_nvidia.so.${version}" /usr/lib/libEGL_nvidia.so.0
install_link "libGLESv2_nvidia.so.${version}" /usr/lib/libGLESv2_nvidia.so.2
install_link "libGLESv1_CM_nvidia.so.${version}" /usr/lib/libGLESv1_CM_nvidia.so.1
install_link "libnvidia-cfg.so.${version}" /usr/lib/libnvidia-cfg.so.1
install_link libnvidia-cfg.so.1 /usr/lib/libnvidia-cfg.so
install_link "libnvidia-allocator.so.${version}" /usr/lib/libnvidia-allocator.so.1
install_link libnvidia-allocator.so.1 /usr/lib/libnvidia-allocator.so
install_link "libnvoptix.so.${version}" /usr/lib/libnvoptix.so.1
install_link "libnvidia-fbc.so.${version}" /usr/lib/libnvidia-fbc.so.1
install_link libnvidia-fbc.so.1 /usr/lib/libnvidia-fbc.so
install_link "libnvcuvid.so.${version}" /usr/lib/libnvcuvid.so.1
install_link libnvcuvid.so.1 /usr/lib/libnvcuvid.so
install_link "libnvidia-encode.so.${version}" /usr/lib/libnvidia-encode.so.1
install_link libnvidia-encode.so.1 /usr/lib/libnvidia-encode.so
install_link "libnvidia-opticalflow.so.${version}" /usr/lib/libnvidia-opticalflow.so.1
install_link libnvidia-opticalflow.so.1 /usr/lib/libnvidia-opticalflow.so
install_link "libnvidia-vksc-core.so.${version}" /usr/lib/libnvidia-vksc-core.so.1
install_link "libnvidia-sandboxutils.so.${version}" /usr/lib/libnvidia-sandboxutils.so.1
install_link libnvidia-sandboxutils.so.1 /usr/lib/libnvidia-sandboxutils.so
install_link_if_exists libnvidia-egl-gbm.so.1.1.3 /usr/lib/libnvidia-egl-gbm.so.1
install_link_if_exists libnvidia-egl-wayland.so.1.1.20 /usr/lib/libnvidia-egl-wayland.so.1
install_link_if_exists libnvidia-egl-wayland2.so.1.0.1 /usr/lib/libnvidia-egl-wayland2.so.1
install_link_if_exists libnvidia-egl-xcb.so.1.0.5 /usr/lib/libnvidia-egl-xcb.so.1
install_link_if_exists libnvidia-egl-xlib.so.1.0.5 /usr/lib/libnvidia-egl-xlib.so.1

install_file 0755 "libvdpau_nvidia.so.${version}" "/usr/lib/vdpau/libvdpau_nvidia.so.${version}"
install_link "libvdpau_nvidia.so.${version}" /usr/lib/vdpau/libvdpau_nvidia.so.1
install_link "vdpau/libvdpau_nvidia.so.${version}" /usr/lib/libvdpau_nvidia.so

install_file 0755 "libglxserver_nvidia.so.${version}" "/usr/lib/xorg/modules/extensions/libglxserver_nvidia.so.${version}"
install_link "libglxserver_nvidia.so.${version}" /usr/lib/xorg/modules/extensions/libglxserver_nvidia.so
install_file 0755 nvidia_drv.so /usr/lib/xorg/modules/drivers/nvidia_drv.so
install_file 0644 nvidia-drm-outputclass.conf /usr/share/X11/xorg.conf.d/nvidia-drm-outputclass.conf

install_link ../libnvidia-allocator.so.1 /usr/lib/gbm/nvidia-drm_gbm.so

install_file 0644 10_nvidia.json /usr/share/glvnd/egl_vendor.d/10_nvidia.json
for json in \
  09_nvidia_wayland2.json \
  10_nvidia_wayland.json \
  15_nvidia_gbm.json \
  20_nvidia_xcb.json \
  20_nvidia_xlib.json; do
  install_file 0644 "${json}" "/usr/share/egl/egl_external_platform.d/${json}"
done

install_file 0644 nvidia_icd.json /usr/share/vulkan/icd.d/nvidia_icd.json
install_file 0644 nvidia_icd_vksc.json /usr/share/vulkan/icd.d/nvidia_icd_vksc.json
install_file 0644 nvidia_layers.json /usr/share/vulkan/implicit_layer.d/nvidia_layers.json
install_file 0644 nvidia.icd /etc/OpenCL/vendors/nvidia.icd

for profile in "${src_dir}"/nvidia-application-profiles-*; do
  if [ -e "${profile}" ]; then
    install -Dm644 "${profile}" "${out_dir}/usr/share/nvidia/$(basename "${profile}")"
  fi
done

install_file 0644 nvoptix.bin /usr/share/nvidia/nvoptix.bin
install_file 0644 sandboxutils-filelist.json /usr/share/nvidia/sandboxutils-filelist.json
install_file 0644 nvidia-settings.desktop /usr/share/applications/nvidia-settings.desktop
install_file 0644 nvidia-settings.png /usr/share/icons/hicolor/128x128/apps/nvidia-settings.png

for firmware in "${src_dir}"/firmware/gsp_*.bin; do
  if [ -e "${firmware}" ]; then
    install -Dm644 "${firmware}" \
      "${out_dir}/usr/lib/firmware/nvidia/${version}/$(basename "${firmware}")"
  fi
done

for unit in "${src_dir}"/systemd/system/*.service; do
  if [ -e "${unit}" ]; then
    install -Dm644 "${unit}" "${out_dir}/usr/lib/systemd/system/$(basename "${unit}")"
  fi
done
install_file 0755 systemd/system-sleep/nvidia /usr/lib/systemd/system-sleep/nvidia

for doc in LICENSE README.txt NVIDIA_Changelog pkg-history.txt; do
  install_file 0644 "${doc}" "/usr/share/doc/nvidia-${version}/${doc}"
done
if [ -d "${src_dir}/html" ]; then
  mkdir -p "${out_dir}/usr/share/doc/nvidia-${version}/html"
  cp -a "${src_dir}/html/." "${out_dir}/usr/share/doc/nvidia-${version}/html/"
fi
if [ -d "${src_dir}/supported-gpus" ]; then
  mkdir -p "${out_dir}/usr/share/doc/nvidia-${version}/supported-gpus"
  cp -a "${src_dir}/supported-gpus/." "${out_dir}/usr/share/doc/nvidia-${version}/supported-gpus/"
fi

find "${out_dir}" -exec touch -h -d "2024-01-01T00:00:00+00:00" {} \;
