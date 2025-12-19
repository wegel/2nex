#!/usr/bin/env python3
"""
allmod.py - controlled allmodconfig with fragment support

usage: ./allmod.py [pre-frags...] -- [post-frags...]

1. starts from x86_64_defconfig
2. applies pre-fragments (gates, builtins)
3. enables all tristates as modules
4. applies post-fragments (disables, overrides)
5. writes .config
"""
import kconfiglib
import os
import sys

def main():
    # set kernel build environment variables
    os.environ.setdefault("srctree", ".")
    os.environ.setdefault("SRCARCH", "x86")
    os.environ.setdefault("ARCH", "x86_64")
    os.environ.setdefault("CC", "gcc")
    os.environ.setdefault("HOSTCC", "gcc")
    os.environ.setdefault("HOSTCXX", "g++")
    os.environ.setdefault("LD", "ld")

    kconf = kconfiglib.Kconfig()

    # 1. start from defconfig
    kconf.load_config("arch/x86/configs/x86_64_defconfig")

    # 2. apply pre-fragments (gates, builtins)
    post = False
    for frag in sys.argv[1:]:
        if frag == '--':
            post = True
            continue
        if not post:
            kconf.load_config(frag, replace=False)

    # 3. enable all tristates as modules (unless already set to =y)
    for sym in kconf.unique_defined_syms:
        if sym.orig_type == kconfiglib.TRISTATE:
            # skip if explicitly set to y (2) by a pre-fragment
            if sym.user_value != 2:
                sym.set_value(1)  # 1 = module

    # 4. apply post-fragments (disables, overrides)
    post = False
    for frag in sys.argv[1:]:
        if frag == '--':
            post = True
            continue
        if post:
            kconf.load_config(frag, replace=False)

    kconf.write_config()

if __name__ == "__main__":
    main()
