# Network Manager patch notes

This directory vendors `pathfinder_simd` 0.5.6 from crates.io so `gpui` 0.2.2 can compile on Apple Silicon with the current stable Rust toolchain used by this project.

## Provenance

- Upstream crate: `pathfinder_simd`
- Version: `0.5.6`
- Source: crates.io registry copy
- Upstream license declaration in source headers: Apache-2.0 or MIT

## Local changes

The local patch is intentionally small:

1. `src/arm/mod.rs`
   - Replaced removed/renamed `std::intrinsics::simd_minimum_number_nsz` and `simd_maximum_number_nsz` calls with the equivalent stable AArch64 NEON numeric min/max intrinsics:
     - `aarch64::vminnm_f32`
     - `aarch64::vmaxnm_f32`
     - `aarch64::vminnmq_f32`
     - `aarch64::vmaxnmq_f32`
2. `src/lib.rs`
   - Suppressed warning noise from the vendored crate (`unexpected_cfgs`, `improper_ctypes`) so project-level checks stay actionable.

## Why this is vendored

`network-manager-ui` uses `gpui` 0.2.2. On Apple Silicon, that dependency currently pulls `pathfinder_simd` 0.5.6, whose AArch64 module references intrinsics no longer available under the current stable Rust names. The patch keeps the UI workpackage buildable without changing GPUI versions mid-implementation.

## Review note

This vendor patch should be removed if either `gpui` updates away from this dependency issue or `pathfinder_simd` publishes a fixed release. Keep this as a separate concern from UI architecture changes in future commits.
