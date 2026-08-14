#!/usr/bin/env python3

import subprocess
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("categorize-modules.py")


class CategorizeModulesTests(unittest.TestCase):
    def run_generator(self, paths):
        return subprocess.run(
            [sys.executable, SCRIPT],
            input="\n".join(paths) + "\n",
            text=True,
            capture_output=True,
            check=False,
        )

    def test_preserves_kernel_sdk_and_non_module_outputs(self):
        result = self.run_generator(
            [
                "100644 regular aaaaaaaa boot/vmlinuz-6.18.24",
                "100644 regular bbbbbbbb boot/System.map-6.18.24",
                "100644 regular cccccccc boot/config-6.18.24",
                "usr/lib/kernel/size/vmlinux.size.txt",
                "120000 symlink dddddddd usr/lib/modules/6.18.24/build",
                "120000 symlink dddddddd usr/lib/modules/6.18.24/source",
                "usr/lib/modules/6.18.24/modules.dep",
                "usr/lib/modules/6.18.24/kernel/drivers/net/virtio_net.ko",
                "usr/lib/modules/6.18.24/kernel/sound/hda/controllers/snd-hda-intel.ko",
                "040000 directory eeeeeeee usr/src/linux-6.18.24/arch/x86",
                "usr/src/linux-6.18.24/.config",
                "usr/src/linux-6.18.24/Module.symvers",
            ]
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("  boot:\n", result.stdout)
        self.assertIn("  drv-net-virtio:\n", result.stdout)
        self.assertIn("  drv-sound-hda:\n", result.stdout)
        self.assertIn("  module-sdk:\n", result.stdout)
        self.assertIn("  modules-meta:\n", result.stdout)
        self.assertIn("  lib:\n", result.stdout)
        self.assertIn(
            "    - path: /usr/src/linux-6.18.24/Module.symvers", result.stdout
        )
        self.assertIn(
            "    - path: /usr/lib/modules/6.18.24/build", result.stdout
        )
        self.assertNotIn("    - path: /usr/src/linux-6.18.24/arch/x86\n", result.stdout)

    def test_rejects_mixed_kernel_releases(self):
        result = self.run_generator(
            [
                "usr/lib/modules/6.18.24/kernel/drivers/net/virtio_net.ko",
                "usr/src/linux-6.19.1/Module.symvers",
            ]
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("mixed kernel releases: 6.18.24 and 6.19.1", result.stderr)


if __name__ == "__main__":
    unittest.main()
