# Notes

## Bootstrap Build Requirements

Bootstrap packages (phases 0-2) must all be built on the same host and in the same path because they have the absolute path of the toolchain hardcoded in them.

For example, you cannot do phase0 on one host and phase1 on another host. Phases 0-2 must be done on a single host with a consistent build path.

Phase3 packages don't have this restriction and can build in parallel on different hosts.
