.PHONY: all builder init build clean

# repository location (can be overridden)
REPO ?= .nex/repo

# default target
all: builder init build

# build the nex builder tool
builder:
	@echo "Building nex builder..."
	cargo build --manifest-path src/builder/Cargo.toml

# initialize the ostree repository
init:
	@echo "Initializing OSTree repository at $(REPO)..."
	@mkdir -p $(dir $(REPO))
	ostree --repo=$(REPO) init --mode=bare-user

# legacy target for compatibility
bootstrap_store:
	$(MAKE) init REPO=bootstrap_store

# build all bootstrap phases
build:
	@echo "Building all phases..."
	@for PHASE in 0 1 2 3; do \
		echo "Phase: $$PHASE"; \
		if [ -d pkg/bootstrap/phase$$PHASE ]; then \
			for M in $$(find pkg/bootstrap/phase$$PHASE -maxdepth 1 -name '*.yaml' | sort -V); do \
				echo "Building: $$M"; \
				B=""; \
				[ $$PHASE -lt 2 ] && B="--bootstrap"; \
				if ! src/builder/target/debug/nex build --repo $(REPO) $$B $$M > /tmp/build_log 2>&1; then \
					cat /tmp/build_log; \
					echo "Failed $$M"; \
					exit 1; \
				fi; \
				rm -rf build_rootfs; \
			done; \
		fi; \
	done
	@echo "Build complete!"

# clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	rm -rf build_rootfs $(REPO)
	cargo clean --manifest-path src/builder/Cargo.toml
