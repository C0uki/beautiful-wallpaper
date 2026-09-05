//! Writes `docs/config.md` from the schema.
//!
//! `cargo run -p bw-core --example config_reference -- docs/config.md`
//!
//! Everything interesting is in [`bw_core::reference`], where it is under test.
//! This is only the part that needs a filesystem.

fn main() {
    let target = std::env::args()
        .nth(1)
        .expect("usage: config_reference <path>");
    let path = std::path::Path::new(&target);

    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory).expect("could not create the output directory");
    }

    std::fs::write(path, bw_core::reference::markdown()).expect("could not write the reference");
    println!("wrote {}", path.display());
}
