# Hashes are computed by vendored rcheevos, not our own code

The old `cheevoshash.c` reimplemented RetroAchievements' hashing by hand and drifted from the official method as consoles were added. We decided to compute every hash by calling the official `rcheevos` C library, vendored as a git submodule pinned to a release tag and compiled into the `hc` binary, so our hashes are by construction the ones RetroAchievements uses.

## Considered Options

- **Pure-Rust reimplementation**: no C toolchain, no submodule, but it repeats the drift problem we are leaving behind.
- **Shelling out to RAHasher**: no interop code, but it adds a runtime dependency users must install and a process spawn per ROM.

## Consequences

- Building needs a C compiler and libclang (for generating the Rust declarations), and cloning needs `--recursive`.
- Supporting a newly added console means bumping the submodule, not writing code.
