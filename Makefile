.PHONY: all cli init clean build-all format-all check-all

# repository location (can be overridden)
REPO ?= .nex/repo

# default target
all: cli init build

# build the nex cli tool
cli:
	@echo "Building nex cli..."
	cargo build --manifest-path src/cli/Cargo.toml

# initialize the zub repository
init:
	@echo "Initializing zub repository at $(REPO)..."
	@mkdir -p $(dir $(REPO))
	zub init $(REPO)

# legacy target for compatibility
bootstrap_store:
	$(MAKE) init REPO=bootstrap_store

clean:
	@echo "Cleaning build artifacts..."
	rm -rf .nex/tmp/* || true
	cargo clean --manifest-path src/cli/Cargo.toml

# build all package manifests sequentially
# usage: make build-all [ARGS="--force --update-checksum"]
build-all:
	@echo "Building all manifests..."
	@for manifest in $$(find pkg -name "*.yaml" | sort); do \
		echo "=== Building $$manifest ==="; \
		./nex build "$$manifest" $(ARGS) || exit 1; \
	done

compute-deps-all:
	@echo "Running compute-deps on all manifests..."
	@for manifest in $$(find pkg -name "*.yaml" | grep -v bootstrap | sort); do \
		echo "=== Building $$manifest ==="; \
		./nex compute-deps "$$manifest" $(ARGS) || exit 1; \
	done

# format all package manifests
format-all:
	@echo "Formatting all manifests..."
	./nex --help >/dev/null
	find pkg -name "*.yaml" | sort | parallel --halt now,fail=1 ./src/cli/target/debug/nex format {}
	@echo "Done formatting all manifests."

# check all package manifests
check-all:
	@echo "Checking all manifests..."
	./nex --help >/dev/null
	find pkg -name "*.yaml" | sort | parallel --halt now,fail=1 ./src/cli/target/debug/nex check {}
	@echo "All manifests passed checks."
