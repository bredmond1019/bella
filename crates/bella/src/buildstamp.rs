//! Build provenance stamp: the `bella --build-stamp` JSON surface.
//!
//! [`build.rs`](../build.rs) stamps three `cargo:rustc-env` values into the binary at
//! compile time (`BELLA_BUILD_GIT_SHA`, `BELLA_BUILD_DIRTY`, `BELLA_BUILD_SOURCE_DIR`); this
//! module exposes them as the pinned `{git_sha, dirty, source_dir}` JSON contract shared with
//! `mev`'s `toolchain-freshness` check and `bastion`'s `src/buildstamp.rs`
//! (`stamp_json_from` at `core/mev/src/brain/conformance/toolchain.rs:296`) — do not add,
//! rename, or drop a key.
//!
//! Scope: this module only builds the stamp. It deliberately does NOT port bastion's
//! drift-verdict types or functions (its `Verdict` enum, its pure verdict computation, its
//! live-HEAD lookup, or its compiled-in-stamp convenience wrapper) — this block registers
//! bella as a freshness *writer*; consuming the verdict is mev's side and is out of scope
//! here.

use serde_json::{Value, json};

const STAMPED_SHA: &str = env!("BELLA_BUILD_GIT_SHA");
const STAMPED_DIRTY: &str = env!("BELLA_BUILD_DIRTY");
const STAMPED_SOURCE_DIR: &str = env!("BELLA_BUILD_SOURCE_DIR");

/// Build the `dirty` JSON value from the raw stamped flag: a JSON boolean when the stamp
/// is `"0"`/`"1"`, or the literal string `"unknown"` when the stamp couldn't be determined
/// at build time — never guessed.
fn dirty_json_value(dirty: &str) -> Value {
    match dirty {
        "0" => Value::Bool(false),
        "1" => Value::Bool(true),
        _ => Value::String("unknown".to_string()),
    }
}

/// Build the `{git_sha, dirty, source_dir}` JSON contract from raw stamp values (pure —
/// exposed for testing independent of the compiled-in `env!` consts).
pub fn stamp_json_from(git_sha: &str, dirty: &str, source_dir: &str) -> Value {
    json!({
        "git_sha": git_sha,
        "dirty": dirty_json_value(dirty),
        "source_dir": source_dir,
    })
}

/// Build the `{git_sha, dirty, source_dir}` JSON contract from this binary's compiled-in
/// stamp. This is the exact shape `bella --build-stamp` prints to stdout.
pub fn stamp_json() -> Value {
    stamp_json_from(STAMPED_SHA, STAMPED_DIRTY, STAMPED_SOURCE_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_json_has_exactly_three_keys() {
        let v = stamp_json_from("abc123", "0", "/src/bella");
        let obj = v.as_object().expect("stamp_json must be a JSON object");
        assert_eq!(obj.len(), 3, "expected exactly 3 keys, got {obj:?}");
        assert!(obj.contains_key("git_sha"));
        assert!(obj.contains_key("dirty"));
        assert!(obj.contains_key("source_dir"));
    }

    #[test]
    fn stamp_json_git_sha_and_source_dir_are_strings() {
        let v = stamp_json_from("abc123", "0", "/src/bella");
        assert_eq!(v["git_sha"], Value::String("abc123".to_string()));
        assert_eq!(v["source_dir"], Value::String("/src/bella".to_string()));
    }

    #[test]
    fn stamp_json_dirty_is_bool_false_for_zero() {
        let v = stamp_json_from("abc123", "0", "/src");
        assert_eq!(v["dirty"], Value::Bool(false));
    }

    #[test]
    fn stamp_json_dirty_is_bool_true_for_one() {
        let v = stamp_json_from("abc123", "1", "/src");
        assert_eq!(v["dirty"], Value::Bool(true));
    }

    #[test]
    fn stamp_json_dirty_is_string_unknown_for_anything_else() {
        let v = stamp_json_from("abc123", "unknown", "/src");
        assert_eq!(v["dirty"], Value::String("unknown".to_string()));
    }

    #[test]
    fn stamp_json_matches_stamp_json_from_with_compiled_in_consts() {
        // stamp_json() must be exactly stamp_json_from() applied to the compiled-in
        // env! consts — no divergence in shape between the two entry points.
        let expected = stamp_json_from(STAMPED_SHA, STAMPED_DIRTY, STAMPED_SOURCE_DIR);
        assert_eq!(stamp_json(), expected);
    }
}
