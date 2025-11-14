.PHONY: all builder init build clean

# default target
all: builder init build

# build the nex builder tool
builder:
	@echo "Building nex builder..."
	cargo build --manifest-path src/builder/Cargo.toml

# initialize the ostree repository
bootstrap_store:
	@echo "Initializing OSTree repository..."
	ostree --repo=bootstrap_store init --mode=bare-user

init: bootstrap_store

# build all bootstrap phases
build:
	@echo "Building all phases..."
	@for PHASE in 0 1 2 3; do \
		echo "Phase: $$PHASE"; \
		for M in $$(ls manifests/bootstrap/phase$$PHASE/*.yaml | sort -V); do \
			echo "Building: $$M"; \
			B=""; \
			[ $$PHASE -lt 2 ] && B="--bootstrap"; \
			if ! src/builder/target/debug/nex $$B bootstrap_store $$M > /tmp/build_log 2>&1; then \
				cat /tmp/build_log; \
				echo "Failed $$M"; \
				exit 1; \
			fi; \
			rm -rf build_rootfs; \
		done; \
	done
	@echo "Build complete!"

# clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	rm -rf build_rootfs bootstrap_store
	cargo clean --manifest-path src/builder/Cargo.toml
