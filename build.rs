fn main() {
    // Tell cargo to recompile when the frontend dist changes.
    // The include_dir! macro embeds frontend/dist at compile time,
    // but cargo doesn't track non-Rust files automatically.
    println!("cargo:rerun-if-changed=frontend/dist");
}
