//! Prints the stylesheet for highlighted code: `cargo run -p mdv-core --example syntax_css`.
fn main() {
    print!("{}", mdv_core::syntax_css());
}
