# Desktop Session Glue

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Package and assemble the desktop session helpers from the OSTreefy parity
matrix. The end state is that `desktop-vwl` has the portal, keyring, policy
agent, terminal, notification, remote desktop, X11 helper, image, font, Vulkan,
and audio helper files that the old daily-driver desktop expected, or the
matrix records a concrete replacement or deferral with proof.

## Progress

- [x] (2026-06-28 17:37Z) Started this sub-EP after completing
  `002d-developer-debug-tools`.
- [x] (2026-06-28 17:37Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-28 17:37Z) Read relevant package-manifest and system-assembly
  knowledge, including Qt platform plugin and `nex_structure` capsule notes.
- [x] (2026-06-28 17:37Z) Ran the new-ExecPlan worktree pre-task. The only
  visible untracked files were local notes/settings files already left
  uncommitted by prior work.
- [x] (2026-06-28 18:10Z) Added existing `qt6-5compat` output to
  `desktop-vwl`. `nex check asm/desktop-vwl/desktop-vwl.yaml` passed, the
  reproducible assembly build passed with checksum
  `3b8adbae70bdc32eb5271504a45f6d9777409fde64ba628fc425ac70d1146ed8`, and a
  checked-out `systems/desktop-vwl/0.0.1` root resolved
  `libQt6Core5Compat.so.6` through the real glibc loader.
- [x] (2026-06-28 18:17Z) Verified existing replacement rows in the checked-out
  `desktop-vwl` root: `vwl` replaces `sway`, `fuzzel` replaces `wofi`,
  `ironbar` replaces `waybar`, and `gnome-text-editor`, `nvim`, and `hx`
  replace `mousepad`.
- [x] (2026-06-28 18:43Z) Added and built `libice` 1.1.2 as the base X11
  library for `xauth` and `xhost`. `nex check` passed, the strict package build
  passed reproducibly with checksum
  `3d5b06cfdb5320da4d66ab6133d4c50ea1f749be1820cbcf256a9ae085080705`, and a
  union-checkout smoke resolved `/usr/lib/libICE.so.6` through glibc.
- [x] (2026-06-28 18:52Z) Added and built `libsm` 1.2.6. `nex check` passed,
  the strict package build passed reproducibly with checksum
  `ff61e6a4514874a2fc49bfcc76006cf01be234d6c2e3024bd104ab2a6644f08e`, and a
  union-checkout smoke resolved `/usr/lib/libSM.so.6` to `libICE.so.6`,
  `libuuid.so.1`, and glibc.
- [x] (2026-06-28 19:02Z) Added and built `libxt` 1.3.1. `nex check` passed,
  the strict package build passed reproducibly with checksum
  `66e091878d302c7ca335f91d6dc318b56f4ab6d97d3c5d2d6d23c55255448a8f`, and a
  union-checkout smoke resolved `/usr/lib/libXt.so.6` through the X11 and
  system libraries it needs.
- [x] (2026-06-28 19:12Z) Added and built `libxmu` 1.3.1. `nex check` passed,
  the strict package build passed reproducibly with checksum
  `e72e52145afc1213b434570d48de1a19a2b7461f4021dcd1690ec2be62b7f90f`, and a
  union-checkout smoke resolved both `/usr/lib/libXmuu.so.1` and
  `/usr/lib/libXmu.so.6`.
- [x] (2026-06-28 19:19Z) Added and built `xauth` 1.1.5. `nex check` passed,
  the strict package build passed reproducibly with checksum
  `4014cbeb207416f56514c12fbe263120139fd5da97f23a20629dff9f23cb84f2`, and a
  union-checkout command smoke printed `1.1.5`.
- [x] (2026-06-28 19:40Z) Fixed `nex resolve` so checksum-addressed
  `.../<checksum>/files` commits process the specific files that pulled them
  into the closure. The focused resolver test passed, `cargo build` passed,
  and `nex resolve xhost` expanded from four commits to seven commits,
  including `libxau`, `libxdmcp`, and `libxcb`.
- [x] (2026-06-28 19:47Z) Added and built `xhost` 1.0.10. `nex check` passed,
  the strict package build passed reproducibly with checksum
  `adbff0f338d5a1c6aade57d8ec8d2e3b59e8edeb6b75b14df4a1e5d042e18d18`, and a
  union-checkout loader smoke resolved `/usr/bin/xhost` through `libXmuu`,
  `libX11`, `libxcb`, `libXau`, `libXdmcp`, glibc, and the loader.
- [x] (2026-06-28 20:04Z) Added `xauth` and `xhost` to `desktop-vwl`. `nex
  check` passed, the reproducible assembly build passed with checksum
  `0879c4bf76b066844db04ea4f0f29853d579c297e7b88447f3cc235195c9e20c`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed `/usr/bin/xauth` and
  `/usr/bin/xhost`; `xauth -V` printed `1.1.5`, and the `xhost` loader smoke
  resolved `libXmuu`, `libX11`, `libxcb`, `libXau`, `libXdmcp`, glibc, and
  the loader.
- [x] (2026-06-28 20:08Z) Skipped `xorg-xeyes` as an X.Org demo app rather
  than a daily-driver program. `rg --files pkg | rg 'libxaw|xeyes'` found no
  existing `xeyes` or Athena widget package, and the matrix row explicitly
  allows skipping this demo app.
- [x] (2026-06-28 20:24Z) Fixed the output categorizer so font files map to
  the `fonts` output instead of `misc`, then added and built `hack-nerd` 3.4.0.
  `nex check` passed, the strict package build passed reproducibly with
  checksum `359f14ce419c6525177c440e24440c88a2247586524b796afd4b6fe349e50245`,
  and a package checkout verified 12 Hack Nerd `.ttf` files including regular,
  mono, and proportional variants.
- [x] (2026-06-28 20:38Z) Added `hack-nerd` to `desktop-vwl` and treated
  `noto-fonts-extra` as covered by the existing universal `noto-fonts`
  package. The reproducible assembly build passed with checksum
  `1071f22d5c1af8544daaf9ef2f01235f650ad379ebb5cecd1dc2f0d7b6354887`, and a
  checked-out system root verified 12 Hack Nerd `.ttf` files plus
  `GoNotoKurrent-Regular.ttf`, `GoNotoCurrentSerif.ttf`, and
  `GoNotoCJKCore.ttf`.
- [x] (2026-06-28 20:46Z) Recorded terminal replacements for `alacritty` and
  `kitty`: `desktop-vwl` already ships `foot`, and `ncurses` already provides
  both Alacritty and Kitty terminfo entries.
- [x] (2026-06-28 21:18Z) Added and built `vulkan-tools` 1.4.328.1. `nex
  check` passed, the strict package build passed reproducibly with checksum
  `73f17af7e8acc6abfc9b2a90d68172f78a9c16c03d3fd76d396aadc0dea888f6`, and a
  union-checkout smoke root ran `vulkaninfo --help`, resolved `vulkaninfo`
  through XCB, X11, Wayland, C++ runtime, glibc, and the loader, resolved
  `libvulkan.so.1`, and ran `vkcube --help`.
- [x] (2026-06-28 21:33Z) Added `vulkan-tools` to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `317ce894ff8d28bcf04bfdea54f5863c188280a016b295b4f06311dad08ad476`, and a
  checked-out `systems/desktop-vwl/0.0.1` root ran public
  `/usr/bin/vulkaninfo --help`, public `/usr/bin/vkcube --help`, resolved the
  package `vulkaninfo` binary through XCB, X11, Wayland, C++ runtime, glibc,
  and the loader, and resolved package `libvulkan.so.1`.
- [x] (2026-06-28 22:50Z) Added and built ImageMagick 7.1.2-26. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `927ba4c6d825b7235bb63bf741c134621f4d438c1e7931ae2a95900bed0bbd72`,
  `nex resolve imagemagick` selected `bundles/full` with a 14-ref runtime
  closure, and a union-checkout smoke root ran `magick -version`, converted a
  generated `xc:red` image to PNG, and identified the resulting PNG.
- [x] (2026-06-28 23:02Z) Added ImageMagick to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `a38b62583005d33306c0c4961133655469092d7079bf1c44f5354ee3638bc731`,
  and a checked-out `systems/desktop-vwl/0.0.1` root exposed public
  `/usr/bin/magick`, ran `magick -version`, converted a generated WebP image,
  and identified the result.
- [x] (2026-06-28 19:00Z) Added and built `gnome-themes-extra` 3.28. `nex
  check` passed, the strict package build passed reproducibly with checksum
  `167c5df9e4d02a7f96dfde719b9568310931dcfcb6346f2df20d2bdd8da79c6c`,
  `nex resolve gnome-themes-extra` selected a one-ref `bundles/dev` closure,
  and a package checkout verified Adwaita, Adwaita-dark, HighContrast, and
  3,456 HighContrast PNG or SVG icon files.
- [x] (2026-06-28 19:03Z) Added `gnome-themes-extra` to `desktop-vwl`. `nex
  check` passed, the reproducible assembly build passed with checksum
  `15b22fb75edac7ffc3b5788f3c466761303a989bfd629ed2abd8e72127762c3a`,
  and a checked-out `systems/desktop-vwl/0.0.1` root linked Adwaita and
  HighContrast files into the `gnome-themes-extra` package capsule with 3,456
  HighContrast PNG or SVG icon files.
- [x] (2026-06-28 19:08Z) Added and built `libvncserver` 0.9.15. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `26ca174aa091ec716771c8919965dab5aaa27b299c4f23886ff1cbe436780325`,
  `nex resolve libvncserver` returned a six-ref closure, and a union-checkout
  smoke resolved both `libvncserver.so.1` and `libvncclient.so.1` through
  zlib, JPEG, PNG, OpenSSL, glibc, and the loader. The smoke also verified
  both pkg-config files report version `0.9.15`.
- [x] (2026-06-28 19:11Z) Added `libvncserver` runtime libraries to
  `desktop-vwl`. `nex check` passed, the reproducible assembly build passed
  with checksum
  `f9fec04510a524a300e28695496993b0967907a4084f15013052ec2b2ba82f26`,
  and a checked-out `systems/desktop-vwl/0.0.1` root linked
  `libvncserver.so.1` and `libvncclient.so.1` into the package capsule and
  resolved both libraries with the real glibc loader.
- [x] (2026-06-28 19:13Z) Marked `swaync` replaced by the existing `mako`
  notification daemon. A checked-out `desktop-vwl` root exposes `/usr/bin/mako`,
  and `mako --help` prints notification daemon options.
- [x] (2026-06-28 20:24Z) Added and built `jack2` 1.9.22 as the JACK server
  and library provider for later JACK command work. `nex check` passed, the
  strict package build passed reproducibly with checksum
  `527b38622c7e290f5b82c6c30427373042583fe7edae9611728f97b2b6e1a096`,
  `nex resolve jack2` returned a five-ref closure, and a union-checkout smoke
  ran `jackd --version`, resolved `libjack.so.0` through the real glibc
  loader, checked `jack.pc` version and linker flags, and verified the
  `jackd` manpage title.
- [x] (2026-06-28 21:02Z) Added and built `libsamplerate` 0.2.2 as a JACK
  tools prerequisite. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `713b82251aa8c4826d30ee160be86a24848e5b5765e7f8632a21cdd10b1f7b9a`,
  `nex resolve libsamplerate` returned a two-ref closure, and a
  union-checkout smoke resolved `libsamplerate.so.0`, checked
  `samplerate.pc`, and ran a compiled consumer through the checked-out glibc
  loader.
- [x] (2026-06-28 21:39Z) Added and built `jack-example-tools` 4. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `99d0cbd66d12a0b0a564883c5ed504130881ef5fbb5606e4ab23729f7ef2cd13`,
  `nex resolve jack-example-tools` returned a ten-ref closure, and a
  union-checkout smoke verified `jack_lsp`, `alsa_in`, `alsa_out`,
  `jack_netsource`, `jack_rec`, `jack_transport`, `jack_server_control`,
  `jack_disconnect`, manpages, representative loader links, and
  `jack_simdtests`.
- [x] (2026-06-28 21:57Z) Added `jack2` and `jack-example-tools` to
  `desktop-vwl`. `nex check` passed, the reproducible assembly build passed
  with checksum
  `3d003f238e2f86ffb3c96a7823c5e14c47b7d2efb7a5dd21e782f0cc786c7a7e`,
  and a checked-out `systems/desktop-vwl/0.0.1` root exposed public
  `/usr/bin/jackd`, `/usr/bin/jack_lsp`, `/usr/bin/alsa_in`,
  `/usr/bin/jack_netsource`, `/usr/bin/jack_rec`, and
  `/usr/bin/jack_disconnect`. The root ran `jackd --version`, resolved
  `jack_transport`, resolved `jack_netsource` through libsamplerate and Opus,
  and ran `jack_simdtests` under `unshare --root` with the real glibc loader.
- [x] (2026-06-28 21:02Z) Added and built FreeRDP 3.27.1 as the X11 RDP
  client provider. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `2360be476465e77dad5ae64b2adbe05f40a299885f3ebbd1c4ff3ea63ccfd32f`,
  `nex resolve freerdp` produced the runtime closure, and a package smoke
  root ran `xfreerdp /version`, checked `WITH_INTERNAL_MD4=ON`, resolved
  `xfreerdp` through FreeRDP, WinPR, X11, OpenSSL, zlib, glibc, and the loader,
  and ran `winpr-hash -u user -p password`.
- [x] (2026-06-28 21:04Z) Added FreeRDP to `desktop-vwl`. `nex check` passed,
  the reproducible assembly build passed with checksum
  `89c5fe6aaa1c90d94aa241cad7dc24d370f43ad5c863cf4e6bd6c8a5b12ded1d`,
  and a checked-out `systems/desktop-vwl/0.0.1` root exposed public
  `/usr/bin/xfreerdp` and `/usr/bin/winpr-hash`. The root ran
  `xfreerdp /version`, verified `WITH_INTERNAL_MD4=ON`, resolved `xfreerdp`
  through the real glibc loader, and ran `winpr-hash` to produce
  `8846f7eaee8fb117ad06bdd830b7586c`.
- [x] (2026-06-28 21:32Z) Added and built `libsigc++` 3.6.0 as the first
  C++ binding dependency for `pavucontrol`. `nex check` passed, the strict
  package build passed reproducibly with checksum
  `24afe09b46b34c92881fc838cde82099d1c03513a77d810eed645f8b6a01ebba`,
  and a union-checkout smoke root resolved `/usr/lib/libsigc-3.0.so.0`
  through libstdc++, libgcc, libc, libm, and the loader. The same root exposed
  `sigc++-3.0.pc`, `sigc++.h`, and reported pkg-config version `3.6.0`.
- [x] (2026-06-28 22:27Z) Added and built `glibmm` 2.84.0 as the GLib and
  GIO C++ binding dependency for `pavucontrol`. `nex check` passed, the strict
  package build passed reproducibly with checksum
  `272855510ed96c6408e8308e1a35cc518178762905f046fa4b54deb9b67f9e28`, and a
  union-checkout smoke root resolved `libglibmm-2.68.so.1` and
  `libgiomm-2.68.so.1` through GLib, GIO, libsigc++, zlib, util-linux
  libraries, the C++ runtime, glibc, and the loader. The same root exposed
  `glibmm.h`, `giomm.h`, `glibmm-2.68.pc`, `giomm-2.68.pc`, and reported
  pkg-config version `2.84.0` for both modules.
- [x] (2026-06-28 22:39Z) Added and built `cairomm` 1.15.4 as the Cairo C++
  binding dependency for `pavucontrol`. `nex check` passed, the strict package
  build passed reproducibly with checksum
  `419336edf43584d47098337293b15dec3062e04156916fbbc007ca2929b15b5b`, and a
  union-checkout smoke root resolved `libcairomm-1.16.so.1` through Cairo,
  FreeType, PNG, X11 libraries, libsigc++, the C++ runtime, glibc, and the
  loader. The same root exposed `cairomm.h`, `cairommconfig.h`,
  `cairomm-1.16.pc`, `cairomm-xlib-1.16.pc`, and reported pkg-config version
  `1.15.4` for both modules.
- [x] (2026-06-28 22:50Z) Added and built `pangomm` 2.56.2 as the Pango C++
  binding dependency for `pavucontrol`. `nex check` passed, the strict package
  build passed reproducibly with checksum
  `a55d2c18288235ef2ccfa3a0fe245b26a678d990ba18b31bddf6a6d46dedcd85`, and a
  union-checkout smoke root resolved `libpangomm-2.48.so.1` through giomm,
  glibmm, cairomm, Pango, Cairo, HarfBuzz, fontconfig, FreeType, GLib,
  libsigc++, the C++ runtime, glibc, and the loader. The same root exposed
  `pangomm.h`, `layout.h`, `pangommconfig.h`, `pangomm-2.48.pc`, and reported
  pkg-config version `2.56.2`.
- [x] (2026-06-28 23:22Z) Added and built `gtkmm` 4.20.0 as the GTK 4 C++
  binding dependency for `pavucontrol`. `nex check` passed, the strict package
  build passed reproducibly with checksum
  `6ab499eecb3b7dd03c65d092c5931539e5e4e004d2f098f61543a6ed100673a6`, and a
  union-checkout smoke root resolved `libgtkmm-4.0.so.0` through giomm,
  glibmm, GTK 4, gdk-pixbuf, Cairo, Graphene, cairomm, pangomm, libsigc++,
  the C++ runtime, glibc, and the loader. The same root exposed `gtkmm.h`,
  `gdkmm.h`, `gskmm.h`, the three generated config headers, `gtkmm-4.0.pc`,
  and reported pkg-config version `4.20.0`.
- [x] (2026-06-28 23:36Z) Added and built `pavucontrol` 6.2 as the PulseAudio
  volume-control GUI. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `48f24a1c258679a102dd3f56993e7b0d2f6d43052151e23704a5f6c6b226545d`,
  and `nex resolve pavucontrol` produced a 44-ref runtime closure including
  `libsndfile`. A fresh union-checkout smoke root verified the command,
  desktop file, metainfo, scalable and symbolic icons, README, style sheet,
  German locale file, and `libsndfile.so.1`; an `unshare --root` loader check
  resolved the binary through GTKmm, GTK, PulseAudio, JSON-GLib, libsndfile,
  GLib, glibmm, libsigc++, the C++ runtime, glibc, and the loader. In the
  headless smoke, `pavucontrol --version` reached GTK and failed only with the
  expected `Failed to open display` warning.
- [x] (2026-06-28 23:42Z) Added `pavucontrol` to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `dcc71cf8aed182738fa6ff67f668429b93f47de049b3745ea7e178647947500a`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed public
  `/usr/bin/pavucontrol` plus the desktop file, metainfo, scalable icon, and
  symbolic icon. The pavucontrol package capsule at
  `/nex/pkg/apps/multimedia/pavucontrol/6.2/fec77fbd` contained the command,
  docs, locale data, and `libsndfile.so.1`; an `unshare --root` loader smoke
  resolved the command from the system root, and the headless command run
  reached GTK before the expected `Failed to open display` warning.
- [x] (2026-06-28 23:58Z) Started the `gnome-keyring` row by adding
  `libgcrypt` 1.11.3 as the missing GCR prerequisite. `nex check` passed, the
  strict package build passed reproducibly with checksum
  `4700f6435161e677faa103056426b2fdcdeee8f0a50b6fbefd2f268fed1309d9`, and a
  union-checkout smoke root resolved `libgcrypt.so.20` through
  `libgpg-error`, ran `hmac256` through the checked-out glibc loader, and
  verified `pkg-config --modversion libgcrypt` reports `1.11.3`.
- [x] (2026-06-29 00:15Z) Added and built `gcr` 3.41.2 as the GCK and
  GCR-base provider for `gnome-keyring`. `nex check` passed, the strict
  package build passed reproducibly with checksum
  `66510bbc0edbf948a3e1702be015b8888195178d604b01efb6bee57efd824b27`, and a
  union-checkout smoke root resolved `libgck-1.so.0`, `libgcr-base-3.so.1`,
  and `gcr-ssh-askpass` through the checked-out glibc loader. The same root
  exposed GCK and GCR-base headers, pkg-config files with version `3.41.2`,
  compiled GLib schemas, and no stale `org.gnome.keyring.*Prompter.service`
  files.
- [x] (2026-06-29 00:45Z) Added and built `gnome-keyring` 50.0 with Secret
  Service, PKCS#11, PAM, and systemd user activation support. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `8c8b4a31f7e8467cdd371054ea8ef6536734a715fe050308fb1ed322af19c26b`, and a
  union-checkout smoke root verified the command symlink, daemon, autostart
  files, D-Bus service files, systemd user unit/socket, Secret portal file,
  p11-kit module, PKCS#11 module, PAM module, internal store modules, GLib
  schemas, locale file, loader resolution, and
  `gnome-keyring-daemon --version`.
- [x] (2026-06-29 00:59Z) Added `gnome-keyring` to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `cb0514609779aa64937684db6c96b599e808265e154471b4a1d2a9a8217d8693`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed public keyring command,
  D-Bus service, and portal symlinks. The keyring capsule contained autostart
  files, service files, systemd user activation files, p11-kit module, PKCS#11
  module, PAM module, internal store modules, source schema XML, and locale
  data; the public schema directory contained `gschemas.compiled`. A loader
  smoke under `unshare --root` ran `gnome-keyring-daemon --version` and
  resolved the daemon, PKCS#11 module, and PAM module.
- [x] (2026-06-28 22:11Z) Added and built `duktape` 2.7.0 as the JavaScript
  engine prerequisite for Polkit 124. `nex check` passed, the strict package
  build passed reproducibly with checksum
  `b8b78f3f9e48c3e9f382d3d58e1d5b2aa39c9d6ec5b1014dfd1d74e79ec7224f`,
  `nex resolve duktape` returned Duktape plus glibc, and a union-checkout
  smoke compiled and ran a small C consumer that evaluated JavaScript through
  `libduktape.so.207`.
- [x] (2026-06-28 22:31Z) Added and built `polkit` 124 with Duktape, PAM, and
  systemd-logind support. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `56827d3309d1d7c90a507419d6fdaccbfa497e848521d0ab01251501d92b2f69`,
  `nex resolve polkit --verbose` returned a 13-ref runtime closure, and a
  package checkout plus dependency-root smoke verified commands, D-Bus files,
  systemd service, sysusers file, PAM file, policy/rules files, setuid helper
  modes, `pkcheck --version`, `pkexec --version`, and loader resolution for
  `polkitd` and `polkit-agent-helper-1`.
- [x] (2026-06-28 22:26Z) Added and built `dbus-glib` 0.112 as the deprecated
  D-Bus GLib binding needed by `policykit-gnome`. `nex check` passed, the
  strict package build passed reproducibly with checksum
  `48a082b8ac22b32ab3e44682936e159455589bb184f4e55dd257ccb06baa161b`,
  `nex resolve dbus-glib --verbose` returned a 12-ref runtime closure, a
  union-checkout smoke generated a client wrapper with `dbus-binding-tool`,
  the loader resolved `libdbus-glib-1.so.2.3.5` through D-Bus, GLib, and
  glibc, and a build-root smoke compiled and ran a C consumer against the
  package headers and library.
- [x] (2026-06-28 22:40Z) Added and built `polkit-gnome` 0.105 as the GTK
  Polkit authentication agent. `nex check` passed, the strict package build
  passed reproducibly with checksum
  `04c3ff3ae759913658b2fa175a9d67945a0f90e7cffc94850b9ca221ac872617`,
  `nex resolve polkit-gnome --verbose` returned a 45-ref runtime closure, and
  a union-checkout smoke verified the agent binary, `libgdk-3.so.0`, and
  `libxkbcommon.so.0`. The real loader resolved the binary through GTK, GDK,
  Polkit, Wayland, X11, GLib, and glibc; running the binary reached GTK and
  failed only with the expected headless `cannot open display:` warning.
- [x] (2026-06-28 22:45Z) Added `polkit-gnome` to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `ac348caf7607f08ca70e9d2984f61a086e9ddedf42dee75c8fd9b2a9ba8b203e`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed the public
  `/usr/libexec/polkit-gnome-authentication-agent-1` symlink. The package
  capsule contained `libpolkit-agent-1.so.0` and `libxkbcommon.so.0`; the real
  loader resolved the agent through the capsule's GTK, GDK, Polkit, Wayland,
  X11, GLib, and glibc libraries. The public command reached GTK and failed
  only with the expected headless `cannot open display:` warning.
- [x] (2026-06-28 23:00Z) Added and built `inih` 61 as a portal-backend
  prerequisite. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `8ed73cfb5352ce50b2797b0285505bedffa2d413af452a05b30b4dc9a347f3e4`,
  `nex resolve inih --verbose` returned a two-ref runtime closure, and a
  union-checkout smoke compiled and ran a small C consumer that parsed an INI
  string through `libinih.so`. The package installs `inih.pc` version `61`.
- [x] (2026-06-28 23:17Z) Started `xdg-desktop-portal-wlr` 0.8.2. The first
  strict package build configured Meson and found PipeWire, Wayland,
  wayland-protocols, inih, GBM, libdrm, and systemd, then failed during C
  compile because `linux/errno.h` was absent. Added `linux-headers` as an
  explicit build-time dependency and added the portal-backend smoke path to
  `.agents/cleanup-workdirs.sh`.
- [x] (2026-06-28 23:31Z) The next strict package build passed reproducibly
  with checksum
  `92f98d798fce30dcf454dd6dfeb493bb4a56441976df84eeceaef0238475c00f`, and
  `nex check` passed. The package smoke then exposed two issues: the `full`
  bundle omitted the generated systemd user service output, and the runtime
  resolver skipped Mesa's `self` edge from `libgbm.so.1` to
  `libgallium-24.2.7.so`, which left `libz.so.1` out of the closure. Began a
  resolver fix instead of adding a fake direct dependency to the portal
  manifest.
- [x] (2026-06-28 23:45Z) Fixed `nex resolve` so checksum-addressed
  dependency `files` commits queue and process same-package `self` files. The
  focused resolver test passed, `cargo fmt --check` passed, and `cargo build`
  passed. The full CLI test suite had one unrelated environment failure:
  `build::orchestration::tests::rebuilds_when_dependency_manifest_changes`
  failed with `uid 0 not mapped in namespace`.
- [x] (2026-06-28 23:53Z) Rebuilt `xdg-desktop-portal-wlr` after adding the
  generated `lib` output to `bundles.full`. The strict package build passed
  reproducibly with checksum
  `92f98d798fce30dcf454dd6dfeb493bb4a56441976df84eeceaef0238475c00f`,
  `nex check` passed, `nex resolve xdg-desktop-portal-wlr --verbose` returned
  a 15-ref closure including Mesa's `libgallium` transitive libraries, and a
  smoke root verified the libexec helper, D-Bus service, systemd user service,
  portal descriptor, help output, and real-loader closure.
- [x] (2026-06-28 23:58Z) Added `xdg-desktop-portal-wlr` to `desktop-vwl`.
  `nex check` passed, the reproducible assembly build passed with checksum
  `5252f03e9bca707d9753a22cc9dd9eb02666135a5ebf3bc758afafb1214b34e4`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed public symlinks for the
  libexec helper, D-Bus service, systemd user service, and portal descriptor.
  The public helper ran `--help` under `unshare --root`, and the real loader
  resolved the package capsule helper through its flattened runtime libraries.
- [x] (2026-06-28 23:11Z) Added and built `fuse3` 3.17.4 as an
  `xdg-desktop-portal` prerequisite. `nex check` passed, the strict package
  build passed reproducibly with checksum
  `b99785309085a14ef0cd91bd31fc95f99631d92b0ffc16a685d1b8f88ca4b950`,
  `nex resolve fuse3 --verbose` returned a two-ref runtime closure, and a
  union-checkout smoke root ran `fusermount3 --version`, resolved
  `fusermount3`, `mount.fuse3`, and `libfuse3.so.4` through the checked-out
  glibc loader, and verified `fuse3.pc`, `fuse.h`, and `/etc/fuse.conf`.
- [x] (2026-06-28 23:18Z) Added and built GStreamer core 1.26.11 as the base
  prerequisite for `gst-plugins-base` and `gstreamer-pbutils-1.0`. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `3d37de67cfde41c29f0ef9da8f2fee72b9f868e24c24512460839f6afb6b54ea`,
  and `nex resolve gstreamer --verbose` returned a seven-ref runtime closure.
  A union-checkout smoke root ran `gst-inspect-1.0 --version`, inspected the
  packaged `coreelements` plugin, ran `gst-launch-1.0 -q fakesrc
  num-buffers=1 ! fakesink`, resolved `libgstcoreelements.so`, and checked
  `gstreamer-1.0.pc` plus `gstreamer-base-1.0.pc`.
- [x] (2026-06-28 23:42Z) Added and built `gst-plugins-base` 1.26.11 as the
  `gstreamer-pbutils-1.0` provider for `xdg-desktop-portal`. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `9f09dd010e2c8e93e475dde0a0f4071163a9fbaa1f25b9d1ff3bdc9615b312fd`,
  and `nex resolve gst-plugins-base --verbose` returned an eight-ref runtime
  closure. A union-checkout smoke root ran `gst-discoverer-1.0 --help`,
  `gst-play-1.0 --help-gst`, and `gst-inspect-1.0 --version`, resolved
  `libgstpbutils-1.0.so.0`, and checked the `gstreamer-pbutils-1.0.pc`
  `Version`, `Requires`, `Libs`, and `Cflags` fields.
- [x] (2026-06-29 00:08Z) Added and built `xdg-desktop-portal` 1.21.2 as the
  core portal frontend. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `4abec7bc20822fbbb7a5cbc1daa620ac0b163dbf3f5320ee17d90db1147d3128`,
  and `nex resolve xdg-desktop-portal --verbose` returned a 21-ref runtime
  closure. A union-checkout smoke root ran `xdg-desktop-portal --help` and
  `--version`, resolved the main portal, document portal, sound validator, and
  icon validator with the checked-out loader, and verified the D-Bus service
  files, systemd user units, pkg-config metadata, and 94 D-Bus interface XML
  files.
- [x] (2026-06-28 23:36Z) Added `xdg-desktop-portal` to `desktop-vwl`.
  `nex check` passed, the reproducible assembly build passed with checksum
  `a80d68adb1e4420141f368c0c5bf43d64909de72b7b0ebb39a257d1321515a92`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed the portal frontend,
  document portal, permission store, wlroots backend, D-Bus service files,
  systemd user units, portal descriptor, D-Bus interface XML, pkg-config
  metadata, and locale files. The root smoke verified service `Exec` and
  `SystemdService` wiring, ran `xdg-desktop-portal --version`, and used the
  real glibc loader to verify all portal libexec binaries.
- [x] (2026-06-29 02:14Z) Added and built `samba` 4.21.3 as the
  `libsmbclient` provider for `gvfs-smb`. `nex check` passed, the strict
  package build passed reproducibly with checksum
  `56af4352a0008c2f20cf06253b2759f88617facadfb0188ade44bd7edd53973a`,
  and `nex resolve samba --verbose` returned a 14-ref runtime closure. A
  union-checkout smoke root verified `libsmbclient.so.0`,
  `libsmbclient.h`, and `smbclient.pc`; a compiled C consumer created and
  freed a `SMBCCTX` through the checked-out library and printed
  `libsmbclient context ok` under `unshare --root`.
- [x] (2026-06-29 02:28Z) Added and built `gvfs` 1.56.1 with the SMB backend
  and keyring support enabled. `nex check` passed, the strict package build
  passed reproducibly with checksum
  `22cfb2574c5c8be1d9c0a2a91d13f82acae3b76223ec30ae3c9c4a0706a87fba`, and
  `nex resolve gvfs --verbose` returned a 19-ref runtime closure including
  Samba and libsecret. A union-checkout smoke root verified the GVFS daemon,
  SMB daemons, D-Bus service files, systemd user units, SMB mount
  descriptors, source and compiled GSettings schemas, and loader closure for
  `gvfsd-smb` and `gvfsd-smb-browse`.
- [x] (2026-06-29 02:39Z) Added `gvfs` to `desktop-vwl`. `nex check` passed,
  the reproducible assembly build passed with checksum
  `35d8129f03666386a8ad065c211c384db672cc3331989074eacb304a61f09c31`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed public GVFS daemon
  symlinks, D-Bus services, systemd user units, SMB mount descriptors, and
  compiled schemas. A real-loader smoke resolved the GVFS package capsule's
  `gvfsd-smb` through `libsmbclient`, libsecret, `libgvfsdaemon`,
  `libgvfscommon`, Samba private libraries, and glibc.
- [x] (2026-06-29 03:30Z) Added and built `qt6-wayland` 6.7.2 as the Qt
  Wayland provider for the old `qt5-wayland` row. `nex check` passed, the
  strict package build passed reproducibly with checksum
  `885de55fc40689afcec7eebbd000501d7459c0ba01882431587c13ff02d67536`, and
  `nex resolve qt6-wayland --verbose` returned a 26-ref runtime closure. A
  union-checkout smoke root verified `libqwayland-generic.so`, xdg shell and
  decoration plugins, `libQt6WaylandClient.so.6`, and
  `libQt6WaylandCompositor.so.6`, then resolved the plugin and library loader
  closures under `unshare --root`.
- [x] (2026-06-29 03:47Z) Added `qt6-wayland` to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `ee54ebc146c8853d4fec54dc1547444f8753a8df7f474df4942fe04addf0411d`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed public symlinks for
  the Wayland platform plugin, shell-integration plugin, decoration plugin,
  `libQt6WaylandClient.so.6`, and `libQt6WaylandCompositor.so.6`. The real
  glibc loader resolved the package capsule's platform plugin, shell plugin,
  and compositor library through the flattened Qt, Wayland, XKB, and C++
  runtime libraries.
- [x] (2026-06-29 04:02Z) Started the `remmina` row by adding `libsodium`
  1.0.22 as a required Remmina core dependency. `nex check` passed, the
  strict package build passed reproducibly with checksum
  `d5785aed721ae43033ec345458c74398538c62294d935b1ea2db0f48012153c7`, and a
  checked-out smoke root verified `sodium.h`, `libsodium.pc` version 1.0.22,
  and a tiny C consumer that called `sodium_init`,
  `crypto_generichash`, and `sodium_version_string` under the checked-out
  glibc loader.
- [x] (2026-06-29 04:49Z) Added and built `remmina` 1.4.43 with GTK3, RDP,
  VNC, libsecret, and exec plugin support. `nex check` passed, the strict
  package build passed reproducibly with checksum
  `fd32ac43d2d1c9542acda179a1446c0e11328db8cbf22f0658117402e52d2715`,
  and `nex resolve remmina --verbose` returned a 54-ref runtime closure that
  includes bash and coreutils for installed shell scripts. A fresh smoke root
  verified the `remmina` binary, file wrapper, desktop file, appdata, MIME
  XML, icon, RDP/VNC/secret/exec plugins, `/usr/bin/sh` shebangs, script
  syntax through the checked-out loader, `remmina --version`, and plugin
  loader links to FreeRDP, libvncclient, libsecret, GTK3, and GDK.
- [x] (2026-06-29 04:52Z) Added `remmina` to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `5f2eca04efd6a013e260cee05f7a1948ddb011c6e15648962620e96c4522603e`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed public symlinks for
  `/usr/bin/remmina`, `/usr/bin/remmina-file-wrapper`, the Remmina desktop
  file, appdata, MIME XML, and RDP/VNC/secret/exec plugins. The real glibc
  loader from the system root resolved the package capsule's `remmina`
  binary and plugins; `remmina --version` printed 1.4.43.
- [x] (2026-06-29 04:55Z) Started the `loupe` row. Loupe 50.0 is too new for
  the current Nex desktop libraries because it requires `libadwaita-1 >=
  1.8.0`, while Nex packages libadwaita 1.7.9. Loupe 49.2 matches
  libadwaita 1.7, but still needs missing `lcms2` and `libgweather-4`
  packages before the app build can start.
- [x] (2026-06-29 05:02Z) Added and built `lcms2` 2.17 as Loupe's color
  management library. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `5ec3a85fa870907ba55274de6ed7600e6357853d258c5465db80fa67d0ec566b`, and a
  smoke root verified `lcms2.pc` version 2.17, `transicc` loader closure, and
  a tiny C consumer that created an sRGB profile and read its profile version
  under the checked-out glibc loader.
- [x] (2026-06-29 05:33Z) Fixed `compute-deps` to resolve absolute
  `DT_NEEDED` entries such as `/usr/lib/libsqlite3.so`. The geocode-glib
  smoke exposed the issue through libsoup3: `readelf -d libsoup-3.0.so.0`
  showed an absolute sqlite dependency, while `nex resolve libsoup3` omitted
  sqlite3. The focused Rust test passed, `cargo fmt --check` passed,
  `cargo build` passed, and the full CLI test suite again failed only in the
  known harness-sensitive namespace test with `uid 0 not mapped in namespace`.
- [x] (2026-06-29 05:38Z) Regenerated libsoup3 runtime metadata with the
  fixed `compute-deps`. `nex check pkg/libs/net/libsoup3.yaml` passed,
  `nex resolve libsoup3 --verbose` returned a 13-ref closure that includes
  sqlite3, and a fresh `.nex/tmp/libsoup3-smoke` union root resolved
  `libsoup-3.0.so.0` through `/usr/lib/libsqlite3.so` with the checked-out
  glibc loader.
- [x] (2026-06-29 05:43Z) Added and built `geocode-glib` 3.26.4 as the
  `geocode-glib-2.0` provider needed by libgweather. The strict package build
  passed reproducibly with checksum
  `b6f59560de37ef0e31ca6e993d55d7655910ad3470ae649269d4d81306d024a6`,
  `nex check` passed, `nex resolve geocode-glib --verbose` returned a 16-ref
  runtime closure including sqlite3 through libsoup3, and a smoke root
  compiled and ran a C consumer that created a `GeocodeLocation` and printed
  `lat 48.8566 lon 2.3522 accuracy 15000`.
- [x] (2026-06-29 06:08Z) Added and built `libgweather` 4.4.4 as Loupe's
  `gweather4` provider. The strict package build passed reproducibly with
  checksum
  `006f2f066d4c07df81e2247fd5bf14834182707341ace68264f57498e5f8a4fd`,
  `nex check` passed, and `nex resolve libgweather --verbose` returned an
  18-ref runtime closure including geocode-glib, libsoup3, sqlite3, libpsl,
  libidn2, libunistring, brotli, and nghttp2. A fresh smoke root verified
  `Locations.bin`, compiled GSettings schemas, `Locations.xml`,
  `gweather4.pc` version 4.4.4, and the real-loader closure for
  `libgweather-4.so.0`. A tiny C consumer compiled against the checked-out
  `gweather4.pc` and ran under `unshare --root`, printing `world (null)`.
- [x] (2026-06-29 06:31Z) Added and built `loupe` 49.2 with vendored Cargo
  dependencies, GTK4, libadwaita 1.7.9, libgweather 4.4.4, lcms2, and
  libseccomp. The strict package build passed reproducibly with checksum
  `4b2b4dcba4a6f45957cc8e385b645c0e89fcddc34d502a01e52bc0152f76622a`,
  `nex check` passed, and `nex resolve loupe --verbose` returned a 52-ref
  runtime closure. A fresh smoke root verified `/usr/bin/loupe`, the desktop
  file, D-Bus service, metainfo, compiled schema, a locale file, and help
  pages. The real loader resolved Loupe through GTK4, libadwaita,
  libgweather, lcms2, libseccomp, libsoup3, sqlite3, and the GTK graphics
  stack; `loupe --help` ran under `unshare --root`.
- [x] (2026-06-29 07:34Z) Added `loupe` to `desktop-vwl`. `nex check` passed,
  the reproducible assembly build passed with checksum
  `f05a3f284d5d170579a5fe9a3b2354b276ea1cb8b3791d77e4dd4f782a5c1104`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed `/usr/bin/loupe`, the
  Loupe desktop file, D-Bus service, metainfo, compiled schema, locale file,
  and help page. The real glibc loader from the system root resolved Loupe's
  package capsule, and `loupe --help` ran under `unshare --root`.

## Surprises & Discoveries

- Observation: The matrix assigns more rows to 002e than the parent plan's
  short target list.
  Evidence: `.agents/ostreefy-parity-matrix.md` assigns terminal packages,
  portals, keyring/polkit helpers, image tools, remote desktop tools, X11
  helpers, font rows, Vulkan tools, JACK/libvncserver helpers, and replacement
  decisions to 002e.
- Observation: The current Nex Qt stack provides the generic Qt Wayland
  platform plugin, but not the EGL-specific Qt Wayland plugin.
  Evidence: `qt6-wayland` 6.7.2 configured with Qt Wayland Client and
  Compositor enabled, but `Wayland EGL` disabled because the current
  `qt6-base` output does not expose the Qt EGL support files that QtWayland
  expects. The package smoke verified `libqwayland-generic.so` and the
  Wayland shell plugins instead of `libqwayland-egl.so`.
- Observation: Remmina 1.4.43 requires libsodium in its core executable even
  when optional remote-desktop plugins are disabled.
  Evidence: `tmp/src-inspect/Remmina-v1.4.43/CMakeLists.txt` calls
  `find_package(sodium REQUIRED)`, and `src/CMakeLists.txt` links `remmina`
  to `sodium` when CMake finds it.
- Observation: A raw checkout of `gcc` plus `binutils` is not enough for an
  ad hoc compiler smoke root.
  Evidence: A libsodium smoke root that included `gcc` and `binutils` bundles
  failed when the checked-out assembler loaded because `libbfd-2.42.so` was
  missing. The successful smoke compiled the tiny C consumer with the host
  compiler against checked-out Nex headers and libraries, then ran it under
  the checked-out Nex glibc loader.
- Observation: Remmina needs FreeRDP's command output at build time, not only
  the FreeRDP development bundle.
  Evidence: Remmina's first CMake configure found FreeRDP 3, then failed in
  `WinPRTargets.cmake` because the imported `winpr-makecert` target pointed
  at `/usr/bin/winpr-makecert`. Adding
  `x86_64/pkg/apps/misc/freerdp/3.27.1/outputs/bin` supplied that command.
- Observation: Current Loupe 50.0 does not match Nex's libadwaita package.
  Evidence: `tmp/src-inspect/loupe-50.0/meson.build` requires
  `libadwaita-1 >= 1.8.0`, but `pkg/libs/graphics/libadwaita.yaml` packages
  libadwaita 1.7.9.
- Observation: Loupe 49.2 matches Nex's libadwaita level but needs missing
  image and GNOME data libraries.
  Evidence: `tmp/src-inspect/loupe-49.2/meson.build` requires `gtk4 >=
  4.16.0`, `libadwaita-1 >= 1.7.0`, `gweather4 >= 4.0.0`, `lcms2 >=
  2.12.0`, and `libseccomp >= 2.5.0`. Nex already packages GTK4,
  libadwaita 1.7.9, and libseccomp, but has no `gweather` or `lcms`
  manifest.
- Observation: Little CMS builds only three command binaries when JPEG and
  TIFF headers are absent.
  Evidence: `pkg/libs/graphics/lcms2.yaml` builds 2.17 with no JPEG, TIFF, or
  zlib inputs. The generated `bin` output contains `linkicc`, `psicc`, and
  `transicc`; the package installs manpages for `jpgicc` and `tificc`, but
  not those binaries.
- Observation: Some CMake options that look optional still trigger dependency
  probes unless disabled directly.
  Evidence: Remmina's RDP plugin build looked for CUPS until the manifest
  passed `-DWITH_CUPS=OFF`, even though the package target for this row only
  needs core Remmina, RDP, VNC, libsecret, and exec plugin support.
- Observation: Package smoke tests for installed shell scripts should execute
  the shell through the checked-out glibc loader.
  Evidence: In the Remmina smoke root, direct `unshare --root
  .nex/tmp/remmina-smoke /usr/bin/sh -n ...` printed `No such file or
  directory` even though `/usr/bin/sh -> bash` and the real loader existed in
  the root. Running `/usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib
  /usr/bin/sh -n ...` under `unshare --root` parsed the scripts correctly.
- Observation: Checked-out `nex_structure` system roots expose
  `/lib64/ld-linux-x86-64.so.2` as the Nex loader shim, not the real glibc
  loader.
  Evidence: A `desktop-vwl` root smoke for `xdg-desktop-portal` failed with
  `nex-ld-shim: .nex-app-root not found for: /lib64/ld-linux-x86-64.so.2`
  when the smoke tried to use `/lib64/ld-linux-x86-64.so.2` as a manual
  loader. Using
  `/nex/pkg/libs/system/glibc/2.39/7fa68fd3/usr/lib/ld-linux-x86-64.so.2`
  inside the checked-out root succeeded.
- Observation: `xdg-desktop-portal` 1.21.2 vendors `libglnx` and `gvdb`.
  Evidence: The release tarball includes `subprojects/libglnx` and
  `subprojects/gvdb`, and Meson configured both subprojects without separate
  Nex packages.
- Observation: `xdg-desktop-portal` can build without Geoclue, GUdev, Flatpak
  interface imports, documentation, tests, man pages, or Bubblewrap validators.
  Evidence: The package manifest disables those Meson features, while the
  build still installs the main portal, document portal, permission store,
  validators, D-Bus service files, systemd user units, locale files, and D-Bus
  interface XML files.
  Evidence: The direct `--list` smoke for `libQt6Core5Compat.so.6` failed
  there with `.nex-app-root not found`; using the real glibc loader under
  `/nex/pkg/libs/system/glibc/2.39/7fa68fd3/usr/lib/ld-linux-x86-64.so.2`
  succeeded.
- Observation: Host-side `-e` checks fail for valid absolute command symlinks
  in a checked-out `nex_structure` root.
  Evidence: `[ -e "$root/usr/bin/vwl" ]` failed because the target begins with
  `/nex/pkg/...`; `[ -L "$root/usr/bin/vwl" ]` passed. Host-side
  `[ -e "$root/usr/libexec/xdg-desktop-portal" ]` failed for the same reason,
  while `unshare --root "$root"` saw the symlink target correctly.
- Observation: Full runtime union checkouts can fail before smoke tests when
  dependency outputs carry config conflicts or hardlink metadata that points
  outside the selected output.
  Evidence: Polkit's full runtime closure first conflicted on
  `/etc/environment`, then failed on glibc hardlink targets such as
  `usr/libexec/getconf/POSIX_V6_LP64_OFF64` and `usr/lib/Scrt1.o`, and also
  hit a systemd type conflict at `/etc/xdg/systemd/user`. The successful smoke
  checked out Polkit's full bundle separately and used the completed build
  root as the dependency library root for loader checks.
- Observation: `polkit-gnome` does not need `dbus-glib` for the current build
  path even though its old source still contains disabled dbus-glib code.
  Evidence: The source references `dbus-glib.h` only inside an `#if 0` block
  in `polkitgnomeauthenticationdialog.c`; the strict build passed without
  `dbus-glib` in the manifest dependencies.
- Observation: GTK3 apps that link only `libgtk-3.so.0` can still need a
  manual `libgdk-3.so.0` output metadata edge for Nex runtime closure smoke.
  Evidence: The first `polkit-gnome` closure had 37 refs and the loader failed
  on missing `libxkbcommon.so.0`. Adding `libgdk-3.so.0` to the executable
  `needs` made the resolver process GTK's GDK metadata, expanded the closure
  to 45 refs, and the loader smoke passed.
- Observation: X.Org release tarballs can still expect `cmp`, `diff`, and
  `xorg-macros.pc` during configure.
  Evidence: The first `libice` build printed missing-tool messages for those
  probes; adding `diffutils` and `util-macros` made the next build materialize
  them.
- Observation: `libSM` needs the `xtrans` headers as a direct build input.
  Evidence: The first build failed on `X11/Xtrans/Xtrans.h`; adding `xtrans`
  fixed the compile.
- Observation: `libXt` needs direct build dependencies for `.pc` files required
  by `sm.pc` and `x11.pc`.
  Evidence: The first `libxt` build failed because `uuid.pc` and `xcb.pc` were
  absent; adding `util_linux`, `libxcb`, `libxau`, and `libxdmcp` fixed the
  configure step.
- Observation: `libXmu` needs direct build dependencies for `.pc` files
  required by `x11.pc` and `xt.pc`.
  Evidence: Builds failed first on missing `xcb.pc`, then on missing `ice.pc`
  and `sm.pc`; direct providers fixed both configure failures.
- Observation: Package-root command smokes may need the explicit loader path.
  Evidence: `xauth` has interpreter `/lib64/ld-linux-x86-64.so.2`, while the
  union root had the loader at `/usr/lib/ld-linux-x86-64.so.2`; invoking the
  explicit loader made `xauth -V` work.
- Observation: `nex resolve` previously stopped at direct dependencies for
  checksum-addressed `files` commits.
  Evidence: Before commit `a6b046e`, `nex resolve xhost` returned only
  `xhost`, glibc, `libx11`, and `libxmu`; a smoke root built from those refs
  failed on missing `libxcb.so.1`. The resolver now finds manifests for
  checksum-addressed `.../<checksum>/files` refs, processes the needed file
  entries in those manifests, and requeues raw file commits when a later file
  path is needed.
- Observation: `xhost` needs gettext at build time.
  Evidence: The first strict `xhost` build failed with `xgettext: command not
  found` while generating `xhost.po`; adding gettext fixed the build.
- Observation: `xhost -help` prints usage but exits with status 1.
  Evidence: The package-root command printed `usage: /usr/bin/xhost
  [[+-]hostname ...]` and returned 1, so the headless smoke uses loader
  resolution instead.
- Observation: The full CLI test suite has an environment-sensitive test in
  this harness.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml` passed 66 tests
  and failed `build::orchestration::tests::rebuilds_when_dependency_manifest_changes`
  with `uid 0 not mapped in namespace`. The focused resolver test and
  `cargo build` passed.
- Observation: `compute-deps` must index absolute library paths as well as
  SONAME basenames.
  Evidence: The geocode-glib smoke root failed to load
  `libgeocode-glib-2.so.0` because libsoup3 had an absolute `DT_NEEDED`
  entry for `/usr/lib/libsqlite3.so`, but `nex resolve libsoup3` did not
  include sqlite3. The scanner now indexes both `libsqlite3.so` and
  `/usr/lib/libsqlite3.so` for provider files.
- Observation: `libgweather` 4 needs Python GI at build time even when
  introspection output is disabled.
  Evidence: The 4.4.4 Meson configure step found `python3 (gi)` and ran
  `build-aux/meson/gen_locations_variant.py`; the manifest therefore lists
  `pygobject` and `gobject-introspection` as explicit build dependencies while
  still passing `-Dintrospection=false` and `-Denable_vala=false`.
- Observation: Host-side compile smokes against a checked-out library can need
  `-Wl,-rpath-link,<root>/usr/lib`.
  Evidence: A tiny `libgweather` consumer compiled with `gweather4.pc` but no
  rpath-link failed because the host linker could not find indirect
  dependencies `libxml2.so.2` and `libgeocode-glib-2.so.0` inside the smoke
  root. Adding `-Wl,-rpath-link,$root_abs/usr/lib` let the same consumer link
  and run under the checked-out loader.
- Observation: Rust packages that use a `cargo_lock` vendor source need
  `gzip` as a build input.
  Evidence: The first strict Loupe build failed before Meson with
  `tar (child): gzip: Cannot exec: No such file or directory` while unpacking
  the builder-generated vendor tarball. Adding the `gzip` package let the
  build unpack `${SOURCE_vendor}`.
- Observation: Loupe 49.2 can use the existing GNOME Rust and C stack when
  X11 support is disabled, but GTK4's pkg-config closure still needs X11
  `.pc` providers.
  Evidence: Meson configured Loupe with `-Dx11=disabled`, found GTK4,
  libadwaita 1.7.9, `gweather4` 4.4.4, lcms2 2.17, and libseccomp 2.5.6, and
  skipped `gtk4-x11`. Before adding `libx11`, `libxext`, `libxrender`,
  `libxcb`, `libxau`, `libxdmcp`, and `xorgproto`, GTK4's dependency check
  failed because Cairo's pkg-config files required X11 and XCB modules.
- Observation: The output generator preserves existing output assignments.
  Evidence: After the first `hack-nerd` build put most `.ttf` files in `misc`,
  adding a font rule to `determine_category` was not enough by itself; the
  generated manifest kept the known paths in `misc` until those paths were
  moved under `fonts` in the manifest.
- Observation: The existing `foot` terminal covers the old Alacritty and Kitty
  terminal rows for practical desktop parity.
  Evidence: The parity matrix now records `alacritty` and `kitty` as replaced
  by `foot`, and `pkg/libs/system/ncurses.yaml` already contains Alacritty and
  Kitty terminfo entries.
- Observation: Vulkan-Tools loads the Vulkan loader dynamically.
  Evidence: The 1.4.328.1 source links `vulkaninfo`, `vkcube`, and `vkcubepp`
  against `Vulkan::Headers` and `${CMAKE_DL_LIBS}` rather than linking
  directly to `libvulkan.so.1`; the manifest therefore records
  `/usr/lib/libvulkan.so.1` as a manual runtime need.
- Observation: The output generator can preserve seeded `needs` entries
  without adding all ELF-linked libraries for the same paths.
  Evidence: After the first successful `vulkan-tools` compile, the generated
  `bin` output kept only the seeded `/usr/lib/libvulkan.so.1` needs. A raw
  output checkout plus `readelf -d` showed the commands also needed glibc,
  GCC runtime libraries, XCB, X11, and Wayland entries.
- Observation: The ImageMagick homepage release archive path for 7.1.2-26 was
  absent, but the GitHub tag archive was usable.
  Evidence:
  `https://imagemagick.org/archive/releases/ImageMagick-7.1.2-26.tar.xz`
  returned 404, while
  `https://github.com/ImageMagick/ImageMagick/archive/refs/tags/7.1.2-26.tar.gz`
  downloaded with sha256
  `d63594e334e1c410f600fb9370d78d49e4dc6f315722ca4ba083e864e5c354cb`.
- Observation: ImageMagick installs libtool archives that Nex does not need.
  Evidence: The package installed `libMagickCore-7.Q16HDRI.la` and
  `libMagickWand-7.Q16HDRI.la`; the manifest deletes `*.la` after install,
  and the generated outputs no longer list `.la` paths.
- Observation: ImageMagick works with a narrower delegate set for this parity
  row.
  Evidence: The build disables X11, Perl, Magick++, OpenMP, and unavailable
  heavyweight delegates, while enabling existing Nex libraries for bzlib,
  fontconfig, freetype, JPEG, LZMA, PNG, TIFF, WebP, XML, zlib, and zstd.
  The package smoke converted an in-memory generated image to PNG and
  identified it.
- Observation: The current `intltool` package installs scripts with a
  `/usr/sbin/perl` shebang, while the Perl package provides `/usr/bin/perl`.
  Evidence: The `gnome-themes-extra` build script creates wrapper scripts for
  `intltool-extract`, `intltool-merge`, `intltool-prepare`,
  `intltool-update`, and `intltoolize` that run `/usr/bin/perl
  /usr/bin/<tool> "$@"`; configure then finds the wrappers under
  `/nex/work/bin` and the strict build passes.
- Observation: Packages that install icon themes can avoid non-reproducible or
  unavailable icon-cache writes by putting a no-op `gtk-update-icon-cache`
  earlier in `PATH`.
  Evidence: `gnome-themes-extra` finds `/nex/work/bin/gtk-update-icon-cache`;
  install prints the upstream "Icon cache not updated" message and the two-pass
  build still produces matching checksum
  `167c5df9e4d02a7f96dfde719b9568310931dcfcb6346f2df20d2bdd8da79c6c`.
- Observation: `gnome-themes-extra` can satisfy the parity row with GTK engine
  builds disabled.
  Evidence: The manifest passes `--disable-gtk2-engine` and
  `--disable-gtk3-engine`, but the package checkout still contains
  `/usr/share/themes/Adwaita/index.theme`,
  `/usr/share/themes/Adwaita/gtk-3.0/gtk.css`,
  `/usr/share/themes/Adwaita-dark/index.theme`,
  `/usr/share/themes/HighContrast/gtk-2.0/gtkrc`,
  `/usr/share/themes/HighContrast/gtk-3.0/gtk.css`, and
  `/usr/share/icons/HighContrast/index.theme` plus 3,456 PNG or SVG icon
  files.
- Observation: Host-side reads of checked-out `nex_structure` theme symlinks
  need target resolution under the checkout root.
  Evidence: In the checked-out `desktop-vwl` root,
  `/usr/share/themes/Adwaita/index.theme` points to
  `/nex/pkg/desktop/themes/gnome-themes-extra/3.28/46cec4f9/usr/share/themes/Adwaita/index.theme`.
  A host `rg` against the public symlink failed with `No such file or
  directory`, while prepending the checkout root to the symlink target verified
  the theme metadata and icon count.
- Observation: LibVNCServer can build the reusable VNC client and server
  libraries without the heavier example app stack.
  Evidence: `pkg/libs/net/libvncserver.yaml` disables examples, tests, SDL,
  GTK, Qt, libsshtunnel, GnuTLS, Gcrypt, systemd, FFmpeg, SASL, and XCB, while
  keeping zlib, JPEG, PNG, OpenSSL TLS, pthreads, WebSockets, and upstream's
  bundled miniLZO fallback. The strict build passes and installs
  `libvncserver.so.1`, `libvncclient.so.1`, headers, CMake files, and
  pkg-config files.
- Observation: GCR 3.41.2 matches `gnome-keyring` 50.0's old library names
  better than the newer GCR 4.x line.
  Evidence: `gnome-keyring` 50.0 requires `gck-1 >= 3.3.4` and
  `gcr-base-3 >= 3.27.90`; GCR 3.41.2 installs those pkg-config files and
  libraries, while GCR 4.x uses different module names.
- Observation: GCR 3.41.2 still asks Meson for `ssh-add` even when
  `-Dssh_agent=false`.
  Evidence: The first strict GCR configure failed in a Meson
  `find_program('ssh-add', required: get_option('ssh_agent')).path()` call.
  Adding the existing OpenSSH 9.9p1 dev bundle supplied `ssh-add` and let the
  base-only build continue.
- Observation: GCR's base-only build installs DBus prompter service files for
  a prompter binary it does not install.
  Evidence: The build uses `-Dgtk=false`, so no `gcr-prompter` binary appears,
  but install created `org.gnome.keyring.PrivatePrompter.service` and
  `org.gnome.keyring.SystemPrompter.service`. The manifest deletes those
  service files, and the smoke root verified they are absent.
- Observation: `gnome-keyring` 50.0 can satisfy the desktop keyring row without
  enabling its SSH-agent option.
  Evidence: The manifest uses upstream's `-Dssh-agent=false` and still installs
  the Secret Service D-Bus files, portal descriptor, PKCS#11 module, PAM
  module, systemd user activation files, and autostart files for the `pkcs11`
  and `secrets` components.
- Observation: The `gnome-keyring` Secret portal file is GNOME-scoped.
  Evidence: `/usr/share/xdg-desktop-portal/portals/gnome-keyring.portal`
  contains `UseIn=gnome`; the package smoke checks that rather than a generic
  default portal key.
- Observation: `gnome-keyring` runtime smoke roots can hit config-file
  conflicts from dependency closures.
  Evidence: The first `zub union-checkout` failed on `/etc/environment`;
  recreating the root with package ref first and `--on-conflict first` let the
  installed-file and loader smoke run.
- Observation: `desktop-vwl` keeps compiled GLib schemas in the public schema
  directory, not always inside the package capsule that provided the schema.
  Evidence: The checked-out keyring capsule did not contain
  `usr/share/glib-2.0/schemas/gschemas.compiled`, while public
  `/usr/share/glib-2.0/schemas/gschemas.compiled` existed and the capsule kept
  `org.gnome.crypto.cache.gschema.xml`.
- Observation: `mako` covers the notification-daemon role from the old
  `swaync` row, but it does not provide SwayNC's notification center UI.
  Evidence: `desktop-vwl` already includes `mako`, and a checked-out system
  root ran `mako --help`. The row is marked as a practical replacement because
  the desktop has a working Wayland notification daemon.
- Observation: The current PipeWire package does not provide PipeWire JACK
  compatibility libraries.
  Evidence: `pkg/libs/audio/pipewire.yaml` passes `-Djack=disabled` and
  `-Dpipewire-jack=disabled`. The matrix row now names `jack2` as the classic
  JACK replacement path unless a future PipeWire JACK package is enabled.
- Observation: JACK2 needs small package-local fixes for the current build
  environment.
  Evidence: The first strict `jack2` build failed because bundled Waf imports
  Python's removed `imp` module under Python 3.12. The manifest patches Waf to
  use `types.ModuleType`. A later build finished but printed
  `fill_template: line 10: sed: command not found`, so `sed` became an
  explicit build input. The final build produced no `sed` warning and passed
  the two-pass checksum check.
- Observation: `jack-example-tools` is separate from JACK2.
  Evidence: The JACK2 source tree has manpages for `jack_lsp`, `jack_connect`,
  and related commands, but it does not build those commands. The
  `jack-example-tools` tag `4` source tree uses Meson and contains the command
  sources under `tools/` and `example-clients/`.
- Observation: Cleanup must go through `.agents/cleanup-workdirs.sh`.
  Evidence: A direct `rm -rf` for `.nex/tmp/jack2-smoke` was corrected by the
  human. The helper now lists the JACK inspection and smoke paths plus the
  downloaded JACK source tarballs, and `bash .agents/cleanup-workdirs.sh`
  removed 10 disposable paths.
- Observation: `libsamplerate` can build the library interface without
  examples or tests.
  Evidence: The manifest passes `-DBUILD_TESTING=OFF`,
  `-DLIBSAMPLERATE_EXAMPLES=OFF`, and `-DLIBSAMPLERATE_INSTALL=ON`. The
  package smoke root contained `samplerate.h`, `samplerate.pc`, CMake package
  files, and `libsamplerate.so.0`; a tiny consumer compiled against the
  checkout and printed `Fastest Sinc Interpolator 1`.
- Observation: The cleanup helper must include smoke paths before checks that
  create them.
  Evidence: The helper now lists the libsamplerate inspection root, smoke
  root, source archive, C smoke source, and C smoke binary. Running
  `bash .agents/cleanup-workdirs.sh` removed 15 disposable paths before the
  libsamplerate smoke.
- Observation: Shell source variables generated from hyphenated source names
  are not safe to reference by name.
  Evidence: The first `jack-example-tools` build used
  `${SOURCE_jack-example-tools}`. Shell parsed that as `SOURCE_jack` with
  default text `example-tools`, so `tar` tried to open `/example-tools`.
  Using `${SOURCE0}` fixed the build.
- Observation: Host-side loader checks can leak through absolute RPATH entries
  in dependency libraries.
  Evidence: `libreadline.so.8` carries `RPATH [/usr/lib]`. A host-side loader
  check for `jack_transport` picked host `/usr/lib/libncursesw.so.6` and
  reported a glibc version mismatch. Running the same loader check under
  `unshare --root .nex/tmp/jack-example-tools-smoke` resolved
  `libncursesw.so.6` from the checked-out root.
- Observation: `jack-example-tools` help and version commands can print the
  requested text but exit with status 1.
  Evidence: `jack_lsp --help`, `jack_lsp --version`, `jack_wait --help`, and
  `jack_netsource --help` printed useful output and returned 1. The smoke uses
  loader checks and `jack_simdtests`, which exits 0, as the passing behavior
  check.
- Observation: `desktop-vwl` flattens runtime libraries into JACK package
  capsules during assembly.
  Evidence: The assembly build printed `Flattened 7 libs into
  libs/audio/jack2/1.9.22/cec088be` and `Flattened 13 libs into
  apps/multimedia/jack-example-tools/4/f72b510f` before writing the
  reproducible system checksum.
- Observation: FreeRDP's NTLM helper needs internal MD4 in this Nex OpenSSL 3
  environment.
  Evidence: A package smoke root first failed `winpr-hash -u user -p
  password` with `OpenSSL LEGACY provider failed to load, no md4 support
  available`; after adding `-DWITH_INTERNAL_MD4=ON`, both the package smoke
  and checked-out `desktop-vwl` smoke produced
  `8846f7eaee8fb117ad06bdd830b7586c`.
- Observation: FreeRDP client channel defaults can pull in extra build-time
  libraries.
  Evidence: CMake enabled the `urbdrc` dynamic channel on Linux and then
  failed on missing `LIBUSB_1_INCLUDE_DIR` and `LIBUSB_1_LIBRARY`; the focused
  X11 client manifest disables it with `-DCHANNEL_URBDRC=OFF`.
- Observation: `zub checkout` and `zub union-checkout` do not take the same
  destination arguments.
  Evidence: `zub --repo .nex/repo checkout --copy --force --destination
  .nex/tmp/desktop-vwl-freerdp-smoke systems/desktop-vwl/0.0.1` failed with
  `unexpected argument '--destination'`; the working form was
  `zub --repo .nex/repo checkout --copy --force systems/desktop-vwl/0.0.1
  .nex/tmp/desktop-vwl-freerdp-smoke`.
- Observation: GTK GUI apps may need a display even for help or version
  options.
  Evidence: In the pavucontrol smoke root, both `pavucontrol --version` and
  `pavucontrol --help` reached GTK and failed with `Failed to open display`.
  The package smoke therefore treats an isolated loader check plus the expected
  no-display failure as the headless proof.
- Observation: `desktop-vwl` flattens the pavucontrol runtime closure into the
  pavucontrol package capsule during assembly.
  Evidence: The build printed `Flattened 62 libs into
  apps/multimedia/pavucontrol/6.2/fec77fbd`. A checked-out system root had
  `/usr/bin/pavucontrol` pointing to that capsule, and an `unshare --root`
  loader check resolved the binary with
  `/nex/pkg/apps/multimedia/pavucontrol/6.2/fec77fbd/usr/lib` plus the
  package's PulseAudio library directory.
- Observation: `libgcrypt` 1.12.x does not match the current Nex
  `libgpg-error` version.
  Evidence: A strict build attempt for `libgcrypt` 1.12.2 failed in configure
  because it requires `libgpg-error >= 1.56`, while Nex currently packages
  `libgpg-error` 1.51 and several existing manifests refer to that versioned
  package. `libgcrypt` 1.11.3 requires `libgpg-error >= 1.49` and builds
  against the current package.
- Observation: `dbus-glib` builds generated examples during `make` even when
  tests are disabled.
  Evidence: The first strict `dbus-glib` 0.112 build reached `dbus/examples`
  and failed because `dbus-binding-tool` invoked `/usr/bin/env python3`.
  Adding the explicit `python3` build dependency let generated marshaller code
  run and let the build continue.
- Observation: The builder names installed gtk-doc HTML as `misc`, not `doc`,
  for `dbus-glib`.
  Evidence: The first `dbus-glib` build that reached output generation wrote
  `bin`, `dev`, `lib`, `man`, and `misc` outputs, then failed because the
  hand-written `full` bundle still referenced `outputs/doc`. Changing the
  bundle to include `misc` matched the generated output names.
- Observation: Current `qt6-base` does not expose EGL support for Qt Wayland.
  Evidence: `rg -n "EGL|egl" pkg/libs/graphics/qt6-base.yaml` found only
  CMake finder and mkspec files, while the `qt6-wayland` configure summary
  reported Wayland EGL as `no`. The package therefore installs
  `libqwayland-generic.so`, not `libqwayland-egl.so`.
- Observation: `qt6-wayland` installs only generated `dev` and `lib` outputs.
  Evidence: The first strict build generated `outputs/dev` and `outputs/lib`,
  then failed during bundle processing because the hand-written bundle also
  referenced missing `outputs/bin` and `outputs/misc`.

## Decision Log

- Decision: Use `.agents/ostreefy-parity-matrix.md` as the source of truth for
  the 002e target set.
  Rationale: The parent plan names an initial list, but the frozen matrix is
  the audited source from the old OSTreefy inputs.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put Vulkan command-line tools under `pkg/apps/graphics` instead
  of `pkg/libs/graphics`.
  Rationale: The package installs user-facing commands, while the Vulkan
  headers, loader, and Mesa ICD stay in library namespaces.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put ImageMagick under `pkg/apps/graphics` and make
  `bundles/full` the command runtime bundle.
  Rationale: ImageMagick provides user-facing commands. The commands need the
  package shared libraries, config files, and locale data together to process
  images from a package or system root.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put `gnome-themes-extra` under `pkg/desktop/themes` with a `dev`
  bundle that includes both `themes` and `misc`.
  Rationale: The package provides desktop theme data and icon files, not
  commands or shared libraries. The generated icon paths land in `misc`, so
  the bundle must include `misc` with the `themes` output to ship the complete
  HighContrast theme.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put `libvncserver` under `pkg/libs/net` and split CMake package
  files into `dev`.
  Rationale: The package provides network protocol libraries. Runtime systems
  need the shared libraries, while downstream builds need the headers,
  pkg-config files, and CMake package files.
  Date/Author: 2026-06-28 / Ralph

- Decision: Add `jack2` under `pkg/libs/audio` before packaging
  `jack-example-tools`.
  Rationale: `jack-example-tools` needs a `jack.pc` provider. The existing
  PipeWire manifest disables JACK compatibility, while JACK2 provides the
  classic JACK server, client libraries, headers, pkg-config metadata, and ALSA
  drivers.
  Date/Author: 2026-06-28 / Ralph

- Decision: Add `libsamplerate` before packaging `jack-example-tools`.
  Rationale: `jack-example-tools` uses libsamplerate for `alsa_in`,
  `alsa_out`, and `jack_netsource`, so a package with libsamplerate gets
  closer to the old desktop JACK helper set than a minimal JACK tools build.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put `jack-example-tools` under `pkg/apps/multimedia`.
  Rationale: The package installs user-facing audio commands and example
  clients, while `jack2` remains the library and server provider under
  `pkg/libs/audio`.
  Date/Author: 2026-06-28 / Ralph

- Decision: Enable `alsa_in_out`, `jack_netsource`, `jack_rec`, Opus, and
  Readline in `jack-example-tools`, but disable `jack_net` and ZALSA.
  Rationale: Current Nex packages provide ALSA, libsamplerate, libsndfile,
  Opus, Readline, and JACK2. Current Nex packages do not provide `libjacknet`,
  `zita-alsa-pcmi`, or `zita-resampler`.
  Date/Author: 2026-06-28 / Ralph

- Decision: Add both `jack2` and `jack-example-tools` to `desktop-vwl`.
  Rationale: `jack-example-tools` supplies the daily-driver commands, while
  `jack2` supplies the `jackd` server and JACK libraries. The current
  PipeWire package disables PipeWire JACK compatibility.
  Date/Author: 2026-06-28 / Ralph

- Decision: Build FreeRDP as a focused X11 client package for desktop parity.
  Rationale: The old system needs an RDP client. The Nex package ships
  `xfreerdp` and WinPR tools while disabling optional Wayland, SDL, server,
  printer, sound, smartcard, Kerberos, FUSE, FFmpeg, Cairo, and USB redirection
  paths that current Nex packages do not need for that client role.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put `pavucontrol` under `pkg/apps/multimedia` and disable
  libcanberra audio feedback.
  Rationale: The package installs a user-facing audio GUI. Current Nex already
  has PulseAudio, JSON-GLib, GTK 4, and the GTKmm stack, but it has no
  libcanberra package; disabling `audio-feedback` keeps the package focused on
  the old desktop volume-control role.
  Date/Author: 2026-06-28 / Ralph

- Decision: Package Duktape before Polkit.
  Rationale: Polkit 124 defaults to Duktape for its JavaScript rules engine.
  Nex does not package SpiderMonkey, and Duktape gives Polkit the needed rules
  engine with a two-ref runtime closure.
  Date/Author: 2026-06-28 / Ralph

- Decision: Package Polkit under `pkg/libs/security` with daemon support.
  Rationale: `polkit-gnome` needs the Polkit agent and GObject libraries, and
  the desktop needs the system authorization daemon, D-Bus files, PAM file,
  systemd service, and sysusers file. The package is a security framework more
  than a user-facing desktop app.
  Date/Author: 2026-06-28 / Ralph

- Decision: Package `dbus-glib` under `pkg/libs/desktop`.
  Rationale: `dbus-glib` provides deprecated GLib bindings and the
  `dbus-binding-tool` compatibility command for old desktop clients such as
  `policykit-gnome`; it is desktop glue rather than a core D-Bus daemon.
  Date/Author: 2026-06-28 / Ralph

- Decision: Package `polkit-gnome` under `pkg/apps/security`.
  Rationale: The package installs a user-session authentication agent that
  prompts the human for Polkit credentials. It is a desktop security app that
  depends on the `polkit` framework but is not itself the core authorization
  daemon.
  Date/Author: 2026-06-28 / Ralph

- Decision: Package `inih` under `pkg/libs/system` with only the C library.
  Rationale: `xdg-desktop-portal-wlr` needs the `inih` C pkg-config provider
  for config parsing. The optional C++ `INIReader` wrapper is not needed for
  that backend and would add an unnecessary C++ runtime edge.
  Date/Author: 2026-06-28 / Ralph

- Decision: Put `xdg-desktop-portal-wlr` under `pkg/desktop/wayland`.
  Rationale: The package is not a general library or end-user application; it
  is a wlroots session backend that owns portal service files and a libexec
  helper for Wayland desktop sessions.
  Date/Author: 2026-06-28 / Ralph

- Decision: Package Samba as a focused client-library provider before GVFS.
  Rationale: GVFS enables SMB support through the `smbclient` pkg-config
  dependency and `libsmbclient.h`. Nex has no smaller SMB client library
  package, and Samba can ship the library and headers without adding server
  commands to the package outputs.
  Date/Author: 2026-06-29 / Ralph

- Decision: Cover the old `qt5-wayland` row with `qt6-wayland`.
  Rationale: Nex already has Qt 6 base and Qt 6 compatibility packages, and it
  has no Qt 5 stack. The old row needs the Qt Wayland platform plugin behavior
  for desktop applications, and `qt6-wayland` supplies that plugin for the
  current Nex Qt stack.
  Date/Author: 2026-06-29 / Ralph

- Decision: Package Remmina with RDP, VNC, libsecret, and exec plugin support
  for the parity row.
  Rationale: The old daily-driver row needs the Remmina remote desktop client.
  RDP and VNC cover the common remote desktop paths already backed by Nex's
  FreeRDP and libvncclient packages. Libsecret preserves desktop credential
  storage. The disabled plugins need extra stacks that the current parity row
  did not require.
  Date/Author: 2026-06-29 / Ralph

## Outcomes & Retrospective

Complete. `desktop-vwl` now covers the 002e desktop-session parity rows with
new packages, existing packages, or documented replacements. The subplan added
or verified the desktop terminal, notification, portal, keyring, Polkit,
graphics, multimedia, remote desktop, X11 helper, font, Qt Wayland, JACK, and
image-viewer pieces named in the parity matrix. Heavy rows that did not match
the current user program set were replaced or skipped with explicit reasons.

The highest-risk fixes were package runtime closure quality and assembly
reproducibility. The subplan fixed absolute `DT_NEEDED` handling in
`compute-deps`, verified rebuilt metadata for affected packages, and kept
`desktop-vwl` reproducible after each coherent assembly batch. The checked-out
system smokes showed that public commands and files resolve through
`nex_structure` package capsules.

Remaining risk: this subplan did not boot the full graphical session. It
proved package builds, assembly builds, checked-out system files, loader
closures, and headless command behavior. A later boot-path plan should test the
whole `desktop-vwl` session under the target runtime.

## Work Plan

Target rows from the matrix:

```text
alacritty
freerdp
gnome-keyring
gnome-themes-extra
gvfs-smb
imagemagick
jack-example-tools
kitty
libvncserver
loupe
mousepad
noto-fonts-extra
pavucontrol
polkit-gnome
qt5-wayland
qt6-5compat
remmina
sway
swaync
ttf-hack-nerd
vulkan-tools
waybar
wofi
xdg-desktop-portal
xdg-desktop-portal-wlr
xorg-xauth
xorg-xeyes
xorg-xhost
```

Work order:

1. Reconcile rows that may already be covered or intentionally replaced:
   `mousepad`, `sway`, `waybar`, `wofi`, and `qt6-5compat`.
2. Package small X11 helpers and fonts first: `xorg-xauth`, `xorg-xhost`,
   `xorg-xeyes`, `ttf-hack-nerd`, and `noto-fonts-extra`.
3. Package standalone desktop tools with narrow dependencies:
   `imagemagick`, `vulkan-tools`, `pavucontrol`, `gnome-themes-extra`,
   `alacritty`, and `kitty`.
4. Package service/session pieces: `xdg-desktop-portal`,
   `xdg-desktop-portal-wlr`, `gnome-keyring`, `polkit-gnome`, and `swaync`.
5. Package remote and filesystem integration pieces: `freerdp`, `remmina`,
   `gvfs-smb`, `libvncserver`, and `jack-example-tools`.
6. Package or replace heavier image-viewer and Qt Wayland pieces: `loupe` and
   `qt5-wayland`.
7. Add completed packages to `asm/desktop-vwl.yaml`, build the assembly
   reproducibly, and smoke commands/files from a checked-out system root after
   each coherent batch.
8. Update `.agents/ostreefy-parity-matrix.md` after each row reaches covered,
   replaced, deferred, or blocked status.

Validation:

- Run `./src/cli/target/debug/nex check <manifest>` for every changed package
  and assembly manifest.
- For package changes, run the strict package command:

```bash
./src/cli/target/debug/nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

- Smoke each new package from a checked-out package root or the built
  `desktop-vwl` root with a command such as `--version`, `--help`, loader
  resolution, desktop/service file checks, or plugin file checks.
- For assembly changes, run:

```bash
./src/cli/target/debug/nex build asm/desktop-vwl.yaml --verbose --check --update-checksum --force
```

- Check the resulting system with:

```bash
zub --repo .nex/repo checkout --copy systems/desktop-vwl/0.0.1 <tmpdir>
```

Then run the smallest non-network command/file checks that prove the added row.

## Recovery Notes

If one package blocks, record the exact missing source, dependency, or design
choice in this sub-EP and the matrix, then continue with independent rows.
Only stop Ralph if the blocker matches `AGENTS.md`.
