#!/bin/sh
# build-missing-closure.sh: one install builds a guest-only leaf before its
# guest-only root, without either exact ref existing in the mounted remote.
set -u

PRIMARY=/nex/store
REMOTE=/run/nex-host-store
WORKTREE=/nex/users/root/manifests
MANIFESTS=/nex/manifests
ENV_PATH=env/ep018-host.yaml
LEAF_PATH=pkg/test/ep018-leaf.yaml
ROOT_PATH=pkg/test/ep018-root.yaml
LEAF_REF=x86_64/pkg/test/ep018-leaf/1.0/outputs/bin
ROOT_REF=x86_64/pkg/test/ep018-root/1.0/outputs/bin
MARKER=EP018_LEAF_MARKER

run_step() {
    label=$1
    shift
    output=$({ "$@"; } 2>&1)
    status=$?
    printf '%s\n' "$output"
    printf '%s-exit=%s\n' "$label" "$status"
    [ "$status" -eq 0 ] || return "$status"
}

assert_ref_absent() {
    store=$1
    ref=$2
    if /usr/bin/zub --repo "$store" rev-parse "$ref" >/dev/null 2>&1; then
        printf 'error-line=unexpected ref in %s: %s\n' "$store" "$ref"
        exit 1
    fi
}

if [ ! -e "$WORKTREE/.git" ]; then
    mkdir -p "$(dirname "$WORKTREE")"
    git -C "$MANIFESTS" worktree add --detach "$WORKTREE" HEAD >/dev/null || exit 1
fi
cd "$WORKTREE" || exit 1
git config user.name 'Nex machine test'
git config user.email 'machine-test@nex.invalid'
mkdir -p "$(dirname "$ENV_PATH")" "$(dirname "$LEAF_PATH")"

cat > "$ENV_PATH" <<'EOF'
name: ep018-host
description: Host-mode environment for the synthetic machine build
execution:
  chroot: false
paths:
  work: nex/work
  out: nex/out
  inputs: inputs
env:
  HOME: /homeless/ep018
  LC_ALL: C
  PATH: "{build_dir}/usr/bin:/usr/bin:/bin"
  WORK_DIR: "{build_dir}/nex/work"
  OUT_DIR: "{build_dir}/nex/out"
EOF
git add "$ENV_PATH"
git commit -q -m 'test: add EP018 host build environment'
env_blob=$(git rev-parse "HEAD:$ENV_PATH") || exit 1

cat > "$LEAF_PATH" <<EOF
package:
  schema: 1
  name: EP018 leaf
  slug: ep018-leaf
  namespace: test
  version: "1.0"
dependencies: []
sources: []
build:
  environment: $env_blob
  script: |
    mkdir -p "\$OUT_DIR/usr/bin"
    {
        echo '#!/bin/sh'
        echo 'echo $MARKER'
    } > "\$OUT_DIR/usr/bin/ep018-leaf"
    chmod 755 "\$OUT_DIR/usr/bin/ep018-leaf"
bundles:
  full:
  - bin
outputs:
  bin:
    files:
    - path: /usr/bin/ep018-leaf
EOF
git add "$LEAF_PATH"
git commit -q -m 'test: add EP018 leaf package'
leaf_commit=$(git rev-parse HEAD) || exit 1

cat > "$ROOT_PATH" <<EOF
package:
  schema: 1
  name: EP018 root
  slug: ep018-root
  namespace: test
  version: "1.0"
dependencies:
- name: ep018-leaf
  commit: $LEAF_REF
  manifest_ref: $leaf_commit
sources: []
build:
  environment: $env_blob
  script: |
    marker=\$(ep018-leaf)
    test "\$marker" = "$MARKER"
    mkdir -p "\$OUT_DIR/usr/bin"
    {
        echo '#!/bin/sh'
        echo "echo \$marker"
    } > "\$OUT_DIR/usr/bin/ep018-root"
    chmod 755 "\$OUT_DIR/usr/bin/ep018-root"
bundles:
  full:
  - bin
outputs:
  bin:
    files:
    - path: /usr/bin/ep018-root
EOF
git add "$ROOT_PATH"
git commit -q -m 'test: add EP018 root package'

for ref in "$LEAF_REF" "$ROOT_REF"; do
    assert_ref_absent "$PRIMARY" "$ref"
    assert_ref_absent "$REMOTE" "$ref"
done
printf 'refs-before=leaf-and-root-absent-from-primary-and-remote\n'

run_step stage nex stage || exit 1
run_step install nex install "$ROOT_PATH" outputs/bin --system || exit 1

build_order=$(printf '%s\n' "$output" |
    sed -n '/^Build order:/,/^Parallel execution plan:/p')
leaf_line=$(printf '%s\n' "$build_order" | grep -n -m1 'ep018-leaf.yaml' | cut -d: -f1)
root_line=$(printf '%s\n' "$build_order" | grep -n -m1 'ep018-root.yaml' | cut -d: -f1)
[ -n "$leaf_line" ] && [ -n "$root_line" ] && [ "$leaf_line" -lt "$root_line" ] || {
    printf 'error-line=%s\n' "build plan did not place leaf before root"
    exit 1
}
printf 'build-order=leaf-before-root\n'

for ref in "$LEAF_REF" "$ROOT_REF"; do
    /usr/bin/zub --repo "$PRIMARY" rev-parse "$ref" >/dev/null || {
        printf 'error-line=guest build did not publish %s\n' "$ref"
        exit 1
    }
    assert_ref_absent "$REMOTE" "$ref"
done
printf 'refs-after=leaf-and-root-only-in-primary\n'

run_step run ep018-root || exit 1
[ "$output" = "$MARKER" ] || {
    printf 'error-line=%s\n' "root command did not print the leaf marker"
    exit 1
}
run_step discard nex discard --force || exit 1

printf 'PASS: build-missing-closure\n'
