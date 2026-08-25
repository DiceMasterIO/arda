//! Shared types, the subseeded PRNG, and the only byte codec in the workspace.
//!
//! See `docs/capstone/01-architecture.md` for the crate boundary rules.

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_builds_and_links() {
        assert_eq!(env!("CARGO_PKG_NAME"), "arda-core");
    }
}
