//! Writes the default config as JSON, so the frontend's mock backend and the
//! settings UI share one source of truth with the Rust schema.
//!
//! `cargo run -p bw-core --example emit_defaults -- <path>`

fn main() {
    let target = std::env::args()
        .nth(1)
        .expect("usage: emit_defaults <output.json>");

    let config = bw_core::Config::default();
    let mut json = serde_json::to_string_pretty(&config).expect("config is serialisable");
    json.push('\n');

    if let Some(parent) = std::path::Path::new(&target).parent() {
        std::fs::create_dir_all(parent).expect("could not create the output directory");
    }
    std::fs::write(&target, json).expect("could not write the defaults");
    println!("wrote {target}");
}
