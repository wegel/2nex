.PHONY: all cli init clean

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
	./zub init $(REPO)

# legacy target for compatibility
bootstrap_store:
	$(MAKE) init REPO=bootstrap_store

clean:
	@echo "Cleaning build artifacts..."
	rm -rf .nex/tmp/* || true
	cargo clean --manifest-path src/cli/Cargo.toml
