use std::fmt::Write as _;

/// The icons the webview surfaces draw, as Lucide SVG bodies by the Material
/// name the code uses. Read here rather than copied, so both bars draw the
/// same set.
const ICONS: &str = "../shell/src/widgets/icons.gen.ts";

fn main() {
    slint_build::compile("ui/bar.slint").expect("the bar's .slint should compile");

    println!("cargo:rerun-if-changed={ICONS}");
    let source = std::fs::read_to_string(ICONS).expect("the icon set should be readable");
    let mut arms = String::new();
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        // `  name:` with the body on the next line, or `  name: '<…>',`.
        let Some(entry) = line.strip_prefix("  ") else {
            continue;
        };
        let Some((name, rest)) = entry.split_once(':') else {
            continue;
        };
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let body = if rest.trim().is_empty() {
            lines.next().unwrap_or_default()
        } else {
            rest
        };
        let (Some(start), Some(end)) = (body.find('\''), body.rfind('\'')) else {
            continue;
        };
        if start < end {
            writeln!(
                arms,
                "        {name:?} => Some(r##\"{}\"##),",
                &body[start + 1..end]
            )
            .unwrap();
        }
    }
    assert!(!arms.is_empty(), "no icons found in {ICONS}");

    let generated = format!(
        "/// The inside of an icon's 24×24 `<svg>`, by name.\n\
         pub fn icon_body(name: &str) -> Option<&'static str> {{\n    match name {{\n{arms}        _ => None,\n    }}\n}}\n"
    );
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("icons.rs");
    std::fs::write(out, generated).expect("the icon table should be writable");
}
