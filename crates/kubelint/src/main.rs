//! The `kubelint` command-line interface.
//!
//! A thin layer over the library crates: parses arguments, discovers files, calls the linter,
//! renders output and picks the exit code. Business logic belongs in the library crates.
//! See `docs/architecture.md`.

fn main() {
    println!(
        "kubelint {} is under construction. See docs/roadmap.md.",
        env!("CARGO_PKG_VERSION")
    );
}
