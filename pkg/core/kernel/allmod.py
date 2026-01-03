#!/usr/bin/env python3
"""
allmod.py - controlled allmodconfig with fragment support

usage: ./allmod.py [pre-frags...] -- [post-frags...]

1. starts from allnoconfig (minimal base)
2. applies pre-fragments (gates, builtins)
3. enables bool gate dependencies for tristates
4. enables all tristates as modules
5. promotes select tristates to =y to satisfy gate bools
6. re-applies pre-fragments to restore =y and =n overrides
7. applies post-fragments (disables, overrides)
8. writes .config
"""
import kconfiglib
import os
import sys

DEBUG_DISABLE = {
    "DEBUG_KERNEL",
    "DEBUG_INFO",
    "DEBUG_INFO_DWARF_TOOLCHAIN_DEFAULT",
    "DEBUG_INFO_BTF",
    "DEBUG_INFO_BTF_MODULES",
    "KASAN",
    "UBSAN",
    "KCSAN",
    "KCOV",
    "LOCKDEP",
    "PROVE_LOCKING",
    "KMEMLEAK",
    "FAULT_INJECTION",
    "SCHED_DEBUG",
    "STACKTRACE",
    "FTRACE",
    "FUNCTION_TRACER",
    "DYNAMIC_DEBUG",
}


def is_debug_like(name: str) -> bool:
    if "DEBUG" in name:
        return True
    if "TEST" in name:
        return True
    if "SELFTEST" in name:
        return True
    if "SANITY" in name:
        return True
    if "WERROR" in name:
        return True
    if name in DEBUG_DISABLE:
        return True
    return False


def parse_fragment(path: str) -> dict:
    explicit = {}
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("##"):
                continue
            if line.startswith("# CONFIG_") and line.endswith("is not set"):
                parts = line.split()
                if len(parts) >= 3:
                    sym = parts[1]
                    if sym.startswith("CONFIG_"):
                        sym = sym[len("CONFIG_"):]
                    explicit[sym] = 0
                continue
            if not line.startswith("CONFIG_"):
                continue
            if "=" not in line:
                continue
            sym, val = line.split("=", 1)
            if sym.startswith("CONFIG_"):
                sym = sym[len("CONFIG_"):]
            if val == "y":
                explicit[sym] = 2
            elif val == "m":
                explicit[sym] = 1
            elif val == "n":
                explicit[sym] = 0
    return explicit


def parse_explicit_settings(argv) -> dict:
    explicit = {}
    for frag in argv:
        if frag == "--":
            continue
        for sym, val in parse_fragment(frag).items():
            explicit[sym] = val
    return explicit


def _bools_in_expr(expr):
    if expr is None:
        return []
    res = []
    for item in kconfiglib.expr_items(expr):
        if isinstance(item, kconfiglib.Symbol) and item.orig_type == kconfiglib.BOOL:
            res.append(item)
    return res


def _tristates_in_expr(expr):
    if expr is None:
        return []
    res = []
    for item in kconfiglib.expr_items(expr):
        if isinstance(item, kconfiglib.Symbol) and item.orig_type == kconfiglib.TRISTATE:
            res.append(item)
    return res


def collect_gate_bools(kconf) -> set:
    gate_bools = set()
    for sym in kconf.unique_defined_syms:
        if sym.orig_type == kconfiglib.TRISTATE:
            gate_bools.update(_bools_in_expr(sym.direct_dep))

    changed = True
    while changed:
        changed = False
        for sym in list(gate_bools):
            for dep in _bools_in_expr(sym.direct_dep):
                if dep not in gate_bools:
                    gate_bools.add(dep)
                    changed = True
    return gate_bools


def collect_default_y_bools(kconf) -> set:
    res = set()
    for sym in kconf.unique_defined_syms:
        if sym.orig_type != kconfiglib.BOOL:
            continue
        for default, _, _ in sym.defaults:
            if default is kconf.y:
                res.add(sym)
                break
    return res


def collect_dangerous_bools(kconf) -> set:
    dangerous = set()
    for sym in kconf.unique_defined_syms:
        if sym.orig_type != kconfiglib.BOOL:
            continue
        for sel_entry in sym.selects:
            # kconfiglib stores (sym, cond, loc) for selects in this kernel tree
            selected = sel_entry[0]
            if isinstance(selected, kconfiglib.Symbol) and selected.orig_type == kconfiglib.TRISTATE:
                dangerous.add(sym)
                break

    changed = True
    while changed:
        changed = False
        for sym in kconf.unique_defined_syms:
            if sym.orig_type != kconfiglib.BOOL or sym in dangerous:
                continue
            for sel_entry in sym.selects:
                selected = sel_entry[0]
                if selected in dangerous:
                    dangerous.add(sym)
                    changed = True
                    break
    return dangerous


def enable_safe_gate_bools(kconf, gate_bools, dangerous_bools, explicit) -> bool:
    changed = False
    for sym in gate_bools:
        name = sym.name
        if not name:
            continue
        if sym in dangerous_bools:
            continue
        if is_debug_like(name):
            continue
        if explicit.get(name) == 0:
            continue
        if sym.str_value == "y":
            continue
        dep = sym.direct_dep
        if dep is None or kconfiglib.expr_value(dep) == 2:
            sym.set_value(2)
            changed = True
    return changed


def _try_promote(dep, tristates) -> bool:
    saved = [(sym, sym.user_value) for sym in tristates]
    for sym in tristates:
        sym.set_value(2)
    ok = kconfiglib.expr_value(dep) == 2
    if not ok:
        for sym, val in saved:
            if val is None:
                sym.unset_value()
            else:
                sym.set_value(val)
    return ok


def promote_tristates_for_gate_bools(
    kconf, candidate_bools, gate_bools, default_y_bools, explicit, promoted
) -> bool:
    changed = False
    for gate in candidate_bools:
        name = gate.name
        if not name:
            continue
        if is_debug_like(name):
            continue
        if explicit.get(name) == 0:
            continue
        if gate.str_value == "y":
            continue

        dep = gate.direct_dep
        if dep is None:
            continue
        clauses = kconfiglib.split_expr(dep, kconfiglib.OR)
        candidate_sets = []
        for clause in clauses:
            tris = []
            for sym in _tristates_in_expr(clause):
                if sym.str_value != "m":
                    continue
                if explicit.get(sym.name) in (0, 1):
                    continue
                if is_debug_like(sym.name):
                    continue
                tris.append(sym)
            candidate_sets.append(tris)
        gate_tags = []
        if gate in gate_bools:
            gate_tags.append("gate")
        if gate in default_y_bools:
            gate_tags.append("default-y")
        gate_tag = "+".join(gate_tags) if gate_tags else "other"

        for tris in sorted(candidate_sets, key=len):
            if not tris:
                continue
            if _try_promote(dep, tris):
                changed = True
                for sym in tris:
                    if sym.str_value == "y":
                        promoted.setdefault(sym.name, set()).add(
                            f"{gate_tag}:{gate.name}"
                        )
                break
    return changed


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

    explicit = parse_explicit_settings(sys.argv[1:])

    # 1. start from allnoconfig (minimal base)
    for sym in kconf.unique_defined_syms:
        sym.set_value(0)  # disable everything

    # 2. apply pre-fragments (gates, builtins)
    post = False
    for frag in sys.argv[1:]:
        if frag == '--':
            post = True
            continue
        if not post:
            kconf.load_config(frag, replace=False)

    gate_bools = collect_gate_bools(kconf)
    candidate_bools = gate_bools
    dangerous_bools = collect_dangerous_bools(kconf)

    # 3. enable all tristates as modules (unless already set to =y)
    for sym in kconf.unique_defined_syms:
        if sym.orig_type == kconfiglib.TRISTATE:
            if sym.user_value != 2:
                sym.set_value(1)  # 1 = module

    # 4. enable safe gate bools whose dependencies are already satisfied
    for _ in range(10):
        if not enable_safe_gate_bools(
            kconf, gate_bools, dangerous_bools, explicit
        ):
            break

    unresolved = []
    for gate in gate_bools:
        name = gate.name
        if not name or gate.str_value == "y":
            continue
        if is_debug_like(name) or explicit.get(name) == 0:
            continue
        dep = gate.direct_dep
        dep_str = kconfiglib.expr_str(dep) if dep is not None else "<none>"
        tag = "dangerous" if gate in dangerous_bools else "gate"
        unresolved.append((name, tag, dep_str))

    if unresolved:
        print(f"unresolved gate bools (still not y): {len(unresolved)}")
        for name, tag, dep_str in sorted(unresolved):
            print(f"  {name} [{tag}] dep: {dep_str}")

    # 6. re-apply pre-fragments to restore =y and =n overrides
    for frag in sys.argv[1:]:
        if frag == '--':
            break
        kconf.load_config(frag, replace=False)

    # 7. apply post-fragments (disables, overrides)
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
