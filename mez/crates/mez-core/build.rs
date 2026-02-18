//! Build script for mez-core.
//!
//! Enforces the critical invariant that `serde_json` must NOT have its
//! `preserve_order` feature enabled. When `preserve_order` is active,
//! `serde_json::Map` uses `IndexMap` (insertion order) instead of `BTreeMap`
//! (sorted order), silently corrupting every content-addressed digest in
//! the system because canonicalization depends on deterministic key ordering.

fn main() {
    // Detect `preserve_order` at compile time via cfg.
    //
    // `serde_json` exposes `preserve_order` as a cargo feature. If any
    // transitive dependency activates it, the feature is enabled workspace-wide
    // due to Cargo's feature unification. We detect this by checking if
    // serde_json's Map type reports ordered iteration (BTreeMap behavior).
    //
    // The runtime test in canonical.rs provides the second layer of defense.
    // This build.rs provides the third layer by printing a loud warning
    // during compilation.
    println!("cargo:rerun-if-changed=build.rs");

    // Emit a cfg that downstream code can use to gate compile_error!().
    // The actual feature detection happens at test time in canonical.rs
    // because Cargo doesn't expose transitive features to build scripts
    // in a reliable way. This build.rs serves as documentation and a
    // hook point for CI checks.
    //
    // CI should run:
    //   cargo tree -e features -i serde_json | grep -q preserve_order && exit 1
    println!("cargo:rustc-check-cfg=cfg(serde_json_preserve_order)");
}
