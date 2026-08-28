//! What the overview shows for a query.
//!
//! The original's search bar is a stack of "search actions" that each decide
//! whether they can answer, and the answers are interleaved by hand. The same
//! shape is kept here, with one deliberate difference: the whole decision is
//! one pure function over inputs the platform layer has already gathered, so
//! the ordering rules — which the user notices immediately when they are
//! wrong — are covered by tests that run on Linux. The Windows half only has
//! to produce a list of applications and a list of windows.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::calc;
use crate::config::Overview;
use crate::dock::WindowInfo;
use crate::search;

/// Prefix that runs the rest of the line as a command.
pub const COMMAND_PREFIX: char = '>';
/// Prefix that addresses the shell itself rather than the machine.
pub const ACTION_PREFIX: char = '/';

/// What kind of row this is, and therefore what activating it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ResultKind {
    /// An arithmetic answer. Activating copies it.
    Calculator,
    /// An open window. Activating brings it forward.
    Window,
    /// An installed application. Activating starts it.
    App,
    /// Something the shell itself does.
    Action,
    /// A command line to run.
    Command,
    /// Hand the query to the configured search engine.
    WebSearch,
}

/// How an application is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AppKind {
    /// A Start-menu shortcut or an executable, started by path.
    Shortcut,
    /// A packaged application, started by its application user model id.
    Packaged,
}

/// One installed application, as the platform layer found it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppEntry {
    pub name: String,
    /// A file path for a shortcut, an application user model id for a
    /// packaged application.
    pub target: String,
    pub kind: AppKind,
    /// Cached PNG path for its icon, or empty.
    pub icon: String,
    /// Shown under the name — where it came from.
    pub subtitle: String,
}

/// One row of the overview's results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LauncherResult {
    pub kind: ResultKind,
    pub title: String,
    pub subtitle: String,
    /// Cached PNG path, or empty when the row draws a glyph instead.
    pub icon: String,
    /// Material Symbols name, drawn when there is no icon.
    pub symbol: String,
    /// What activating this row acts on: a window id, an application target,
    /// a command line, a URL, or an action keyword.
    pub payload: String,
    /// How to start it, on an application row and nowhere else.
    ///
    /// A packaged application and a shortcut are both a string in `payload`
    /// and are started by entirely different mechanisms, so which one it is
    /// travels with the row rather than being guessed from its shape.
    pub app_kind: Option<AppKind>,
    /// Which characters of the title the query matched, for highlighting.
    ///
    /// Character indices, not bytes: the frontend has to index with
    /// `Array.from(title)`.
    pub positions: Vec<usize>,
}

/// One thing the shell can be told to do from the search bar.
pub struct Action {
    /// What the user types after the `/`. Deliberately English: this is a
    /// command name, not prose, and the frontend supplies the description in
    /// the user's own language.
    pub keyword: &'static str,
    pub symbol: &'static str,
}

/// Every `/` action, in the order they are offered.
///
/// Each one maps to something the shell already does; nothing here is a
/// promise of a feature that does not exist yet.
pub const ACTIONS: &[Action] = &[
    Action {
        keyword: "light",
        symbol: "light_mode",
    },
    Action {
        keyword: "dark",
        symbol: "dark_mode",
    },
    Action {
        keyword: "wallpaper",
        symbol: "wallpaper",
    },
    Action {
        keyword: "random",
        symbol: "shuffle",
    },
    Action {
        keyword: "widgets",
        symbol: "widgets",
    },
    Action {
        keyword: "sidebar",
        symbol: "right_panel_open",
    },
    Action {
        keyword: "screenshot",
        symbol: "photo_camera",
    },
    Action {
        keyword: "ocr",
        symbol: "text_fields",
    },
    Action {
        keyword: "translate",
        symbol: "translate",
    },
];

/// How a query is being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Search,
    Command,
    Action,
}

/// Splits a query into how to read it and what is left after the prefix.
pub fn parse(query: &str) -> (Mode, &str) {
    let trimmed = query.trim_start();
    if let Some(rest) = trimmed.strip_prefix(COMMAND_PREFIX) {
        return (Mode::Command, rest.trim_start());
    }
    if let Some(rest) = trimmed.strip_prefix(ACTION_PREFIX) {
        return (Mode::Action, rest.trim_start());
    }
    (Mode::Search, query.trim())
}

/// Everything the overview should show, in the order it shows it.
pub fn results(
    query: &str,
    windows: &[WindowInfo],
    apps: &[AppEntry],
    config: &Overview,
) -> Vec<LauncherResult> {
    let (mode, rest) = parse(query);

    // A prefix is a statement of intent. Someone who typed `>` wants to run
    // something, and burying that under fuzzy-matched applications would be
    // ignoring what they said.
    match mode {
        Mode::Command => return command_results(rest, config),
        Mode::Action => return action_results(rest),
        Mode::Search => {}
    }

    let mut rows: Vec<LauncherResult> = Vec::new();

    if let Some(answer) = calculator_row(rest) {
        rows.push(answer);
    }

    let limit = config.max_results.max(1) as usize;
    let mut matched: Vec<LauncherResult> = Vec::new();

    if config.show_windows {
        let titles: Vec<&str> = windows.iter().map(|window| window.title.as_str()).collect();
        for (index, found) in search::rank(rest, titles) {
            let window = &windows[index];
            matched.push(LauncherResult {
                kind: ResultKind::Window,
                title: window.title.clone(),
                subtitle: window.name.clone(),
                icon: window.icon.clone(),
                symbol: "select_window".to_owned(),
                payload: window.id.clone(),
                app_kind: None,
                positions: found.positions,
            });
        }
    }

    // An empty query is the overview being opened rather than searched, and
    // what is useful then is what is already running — not a list of every
    // application on the machine in alphabetical order.
    if config.show_apps && !rest.is_empty() {
        let names: Vec<&str> = apps.iter().map(|app| app.name.as_str()).collect();
        for (index, found) in search::rank(rest, names) {
            let app = &apps[index];
            matched.push(LauncherResult {
                kind: ResultKind::App,
                title: app.name.clone(),
                subtitle: app.subtitle.clone(),
                icon: app.icon.clone(),
                symbol: "apps".to_owned(),
                payload: app.target.clone(),
                app_kind: Some(app.kind),
                positions: found.positions,
            });
        }
    }

    matched.truncate(limit);
    rows.append(&mut matched);

    // Always last, and always there: a query that matched nothing still has
    // somewhere to go, which is the difference between a launcher that failed
    // and one that is broken.
    if !rest.is_empty() {
        rows.push(LauncherResult {
            kind: ResultKind::WebSearch,
            title: rest.to_owned(),
            subtitle: String::new(),
            icon: String::new(),
            symbol: "travel_explore".to_owned(),
            payload: web_search_url(&config.search_engine, rest),
            app_kind: None,
            positions: Vec::new(),
        });
    }

    rows
}

fn calculator_row(query: &str) -> Option<LauncherResult> {
    if !calc::looks_like_expression(query) {
        return None;
    }
    let answer = calc::format(calc::evaluate(query)?);
    Some(LauncherResult {
        kind: ResultKind::Calculator,
        title: answer.clone(),
        subtitle: query.trim().to_owned(),
        icon: String::new(),
        symbol: "calculate".to_owned(),
        payload: answer,
        app_kind: None,
        positions: Vec::new(),
    })
}

fn command_results(rest: &str, config: &Overview) -> Vec<LauncherResult> {
    if !config.allow_run_command || rest.is_empty() {
        return Vec::new();
    }
    vec![LauncherResult {
        kind: ResultKind::Command,
        title: rest.to_owned(),
        subtitle: String::new(),
        icon: String::new(),
        symbol: "terminal".to_owned(),
        payload: rest.to_owned(),
        app_kind: None,
        positions: Vec::new(),
    }]
}

fn action_results(rest: &str) -> Vec<LauncherResult> {
    let keywords: Vec<&str> = ACTIONS.iter().map(|action| action.keyword).collect();
    search::rank(rest, keywords)
        .into_iter()
        .map(|(index, found)| {
            let action = &ACTIONS[index];
            LauncherResult {
                kind: ResultKind::Action,
                title: action.keyword.to_owned(),
                subtitle: String::new(),
                icon: String::new(),
                symbol: action.symbol.to_owned(),
                payload: action.keyword.to_owned(),
                app_kind: None,
                positions: found.positions,
            }
        })
        .collect()
}

/// Splits a command line into the program to run and its arguments.
///
/// Windows starts a program by file and parameters, not by one string, so the
/// line the user typed has to be taken apart — and taken apart the way they
/// meant it. Splitting on the first space alone turns
/// `"C:\\Program Files\\app.exe" --flag` into a request to run
/// `C:\\Program`, which fails with an error naming a path nobody typed.
pub fn split_command(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(rest) = line.strip_prefix('"') {
        let (program, arguments) = rest.split_once('"')?;
        if program.is_empty() {
            return None;
        }
        return Some((program.to_owned(), arguments.trim().to_owned()));
    }

    match line.split_once(char::is_whitespace) {
        Some((program, arguments)) => Some((program.to_owned(), arguments.trim().to_owned())),
        None => Some((line.to_owned(), String::new())),
    }
}

/// Fills a search-engine template with the query.
///
/// The query is percent-encoded rather than pasted in raw. Searching for
/// `rust & c++` with a raw substitution loses everything from the ampersand
/// on, and the user gets results for a phrase they did not type.
pub fn web_search_url(template: &str, query: &str) -> String {
    let encoded = percent_encode(query);
    if template.contains("%s") {
        return template.replace("%s", &encoded);
    }
    // A template without a placeholder is still usable as a prefix, which is
    // what a hand-edited config is most likely to contain.
    format!("{template}{encoded}")
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            b' ' => encoded.push('+'),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: &str, title: &str, name: &str) -> WindowInfo {
        WindowInfo {
            id: id.to_owned(),
            title: title.to_owned(),
            executable: format!("c:\\{name}.exe"),
            name: name.to_owned(),
            icon: String::new(),
            active: false,
        }
    }

    fn app(name: &str) -> AppEntry {
        AppEntry {
            name: name.to_owned(),
            target: format!("C:\\Programs\\{name}.lnk"),
            kind: AppKind::Shortcut,
            icon: String::new(),
            subtitle: "Start menu".to_owned(),
        }
    }

    fn config() -> Overview {
        Overview::default()
    }

    #[test]
    fn a_prefix_decides_how_the_rest_is_read() {
        assert_eq!(parse("> ping"), (Mode::Command, "ping"));
        assert_eq!(parse("/dark"), (Mode::Action, "dark"));
        assert_eq!(parse("  notepad  "), (Mode::Search, "notepad"));
    }

    /// Someone who typed `>` said what they wanted; applications must not be
    /// mixed into the answer.
    #[test]
    fn the_command_prefix_offers_only_the_command() {
        let rows = results(">ipconfig", &[], &[app("Notepad")], &config());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, ResultKind::Command);
        assert_eq!(rows[0].payload, "ipconfig");
    }

    #[test]
    fn the_action_prefix_offers_only_actions() {
        let rows = results("/dark", &[], &[app("Notepad")], &config());
        assert!(rows.iter().all(|row| row.kind == ResultKind::Action));
        assert_eq!(rows[0].payload, "dark");
    }

    #[test]
    fn a_bare_slash_lists_every_action() {
        let rows = results("/", &[], &[], &config());
        assert_eq!(rows.len(), ACTIONS.len());
    }

    #[test]
    fn running_a_command_can_be_switched_off() {
        let mut config = config();
        config.allow_run_command = false;
        assert!(results(">ipconfig", &[], &[], &config).is_empty());
    }

    #[test]
    fn arithmetic_comes_first_and_a_web_search_comes_last() {
        let rows = results("2 + 2", &[], &[app("Notepad")], &config());
        assert_eq!(rows[0].kind, ResultKind::Calculator);
        assert_eq!(rows[0].title, "4");
        assert_eq!(rows.last().expect("a row").kind, ResultKind::WebSearch);
    }

    #[test]
    fn open_windows_are_offered_before_applications() {
        let windows = [window("1", "notes.txt — Notepad", "Notepad")];
        let rows = results("notepad", &windows, &[app("Notepad")], &config());
        let kinds: Vec<ResultKind> = rows.iter().map(|row| row.kind).collect();
        assert_eq!(
            kinds,
            vec![ResultKind::Window, ResultKind::App, ResultKind::WebSearch]
        );
    }

    /// Opening the overview should show what is running, not every program
    /// installed on the machine.
    #[test]
    fn an_empty_query_lists_windows_but_not_applications_or_a_web_search() {
        let windows = [window("1", "notes.txt — Notepad", "Notepad")];
        let rows = results("", &windows, &[app("Notepad")], &config());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, ResultKind::Window);
    }

    #[test]
    fn a_query_that_matches_nothing_still_offers_a_web_search() {
        let rows = results("qqqq", &[], &[app("Notepad")], &config());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, ResultKind::WebSearch);
    }

    #[test]
    fn the_result_limit_caps_the_matches_without_swallowing_the_web_search() {
        let mut config = config();
        config.max_results = 1;
        let apps: Vec<AppEntry> = ["Calculator", "Calendar", "Camera"]
            .iter()
            .map(|name| app(name))
            .collect();

        let rows = results("ca", &[], &apps, &config);
        assert_eq!(
            rows.iter()
                .filter(|row| row.kind == ResultKind::App)
                .count(),
            1
        );
        assert_eq!(rows.last().expect("a row").kind, ResultKind::WebSearch);
    }

    /// The answer is one row and it is the reason the user typed; the limit is
    /// about how long the list of programs gets.
    #[test]
    fn the_result_limit_never_hides_the_arithmetic_answer() {
        let mut config = config();
        config.max_results = 1;
        let apps: Vec<AppEntry> = ["Calculator", "Calendar"]
            .iter()
            .map(|name| app(name))
            .collect();

        let rows = results("2 * 3", &[], &apps, &config);
        assert_eq!(rows[0].kind, ResultKind::Calculator);
        assert_eq!(rows[0].title, "6");
    }

    #[test]
    fn windows_and_applications_can_each_be_switched_off() {
        let windows = [window("1", "notes.txt — Notepad", "Notepad")];
        let apps = [app("Notepad")];

        let mut without_windows = config();
        without_windows.show_windows = false;
        assert!(results("notepad", &windows, &apps, &without_windows)
            .iter()
            .all(|row| row.kind != ResultKind::Window));

        let mut without_apps = config();
        without_apps.show_apps = false;
        assert!(results("notepad", &windows, &apps, &without_apps)
            .iter()
            .all(|row| row.kind != ResultKind::App));
    }

    /// A packaged application and a shortcut are both a string in `payload`,
    /// and starting one the other's way does nothing at all.
    #[test]
    fn an_application_row_carries_how_to_start_it() {
        let packaged = AppEntry {
            name: "Terminal".to_owned(),
            target: "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App".to_owned(),
            kind: AppKind::Packaged,
            icon: String::new(),
            subtitle: "Microsoft Store".to_owned(),
        };

        let rows = results(
            "terminal",
            &[],
            &[packaged, app("Terminal Classic")],
            &config(),
        );
        let kinds: Vec<Option<AppKind>> = rows
            .iter()
            .filter(|row| row.kind == ResultKind::App)
            .map(|row| row.app_kind)
            .collect();
        assert_eq!(
            kinds,
            vec![Some(AppKind::Packaged), Some(AppKind::Shortcut)]
        );

        // Nothing else claims to be startable that way.
        assert!(rows
            .iter()
            .filter(|row| row.kind != ResultKind::App)
            .all(|row| row.app_kind.is_none()));
    }

    /// Splitting on the first space would try to run `C:\\Program`.
    #[test]
    fn a_quoted_program_keeps_the_spaces_in_its_path() {
        assert_eq!(
            split_command("\"C:\\Program Files\\app.exe\" --flag"),
            Some(("C:\\Program Files\\app.exe".to_owned(), "--flag".to_owned()))
        );
    }

    #[test]
    fn an_unquoted_command_splits_at_the_first_space() {
        assert_eq!(
            split_command("ping 8.8.8.8 -t"),
            Some(("ping".to_owned(), "8.8.8.8 -t".to_owned()))
        );
        assert_eq!(
            split_command("notepad"),
            Some(("notepad".to_owned(), String::new()))
        );
    }

    #[test]
    fn a_command_that_is_nothing_to_run_is_refused() {
        assert!(split_command("").is_none());
        assert!(split_command("   ").is_none());
        // A quote the user has not closed yet.
        assert!(split_command("\"C:\\Program Files").is_none());
        assert!(split_command("\"\" --flag").is_none());
    }

    /// A raw substitution loses everything after the ampersand.
    #[test]
    fn a_web_search_encodes_the_query() {
        let url = web_search_url("https://example.com/search?q=%s", "rust & c++");
        assert_eq!(url, "https://example.com/search?q=rust+%26+c%2B%2B");
    }

    #[test]
    fn a_template_without_a_placeholder_is_used_as_a_prefix() {
        let url = web_search_url("https://example.com/?q=", "hello world");
        assert_eq!(url, "https://example.com/?q=hello+world");
    }

    #[test]
    fn a_multibyte_query_is_encoded_by_bytes() {
        let url = web_search_url("https://example.com/?q=%s", "壁紙");
        assert_eq!(url, "https://example.com/?q=%E5%A3%81%E7%B4%99");
    }

    #[test]
    fn highlight_positions_index_the_title_that_is_shown() {
        let rows = results("code", &[], &[app("VS Code")], &config());
        let row = rows
            .iter()
            .find(|row| row.kind == ResultKind::App)
            .expect("an application row");
        let characters: Vec<char> = row.title.chars().collect();
        let matched: String = row
            .positions
            .iter()
            .map(|index| characters[*index])
            .collect();
        assert_eq!(matched.to_lowercase(), "code");
    }
}
