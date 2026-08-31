//! Writes the values the frontend has to agree with the Rust schema about, so
//! there is one source of truth rather than two hand-kept copies.
//!
//! `cargo run -p bw-core --example emit_defaults -- <directory>`
//!
//! Two files come out. `defaultConfig.json` is what the mock backend and the
//! settings UI start from. `launcherActions.json` is the list of `/` keywords
//! the launcher offers — the frontend has to know what each one *does*, and a
//! keyword added here with nothing behind it there is a row that appears in
//! the list and does nothing when it is chosen.

fn main() {
    let target = std::env::args()
        .nth(1)
        .expect("usage: emit_defaults <directory>");
    let directory = std::path::Path::new(&target);
    std::fs::create_dir_all(directory).expect("could not create the output directory");

    let config = bw_core::Config::default();
    write(
        &directory.join("defaultConfig.json"),
        &serde_json::to_string_pretty(&config).expect("config is serialisable"),
    );

    let keywords: Vec<&str> = bw_core::launcher::ACTIONS
        .iter()
        .map(|action| action.keyword)
        .collect();
    write(
        &directory.join("launcherActions.json"),
        &serde_json::to_string_pretty(&keywords).expect("keywords are serialisable"),
    );
}

fn write(path: &std::path::Path, contents: &str) {
    std::fs::write(path, format!("{contents}\n")).expect("could not write the generated file");
    println!("wrote {}", path.display());
}
