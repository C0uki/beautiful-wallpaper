//! The notification banners, as the webview's `Toasts.tsx` draws them.
//!
//! Grouped by the application that sent them, the newest of each on screen
//! with a count of the rest; gone by themselves when their time is up, except
//! a critical one; closed or swiped away, which dismisses them for good. One
//! window each, laid one under another from the corner the config names.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use serde::Deserialize;
use slint::ComponentHandle;

use crate::{icon, round_corners, screen_size, settle, window_of, Action, Toast};

/// What the banners need from the config.
#[derive(Debug, Clone)]
pub struct ToastSettings {
    /// How long a banner stays up, in milliseconds.
    pub timeout: u32,
    /// `top_right`, `bottom_left`, `top_center` and so on.
    pub position: String,
    pub max_visible: u32,
    pub do_not_disturb: bool,
    /// Whether the bar is along the top, so the banners keep clear of it.
    pub bar: bool,
    /// Whether the shell's own notices are shown in Japanese.
    pub japanese: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Notification {
    id: u32,
    app_name: String,
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    urgency: String,
    /// Seconds since the epoch.
    time: u64,
}

/// A banner on screen.
struct Shown {
    id: u32,
    window: Toast,
    /// Hides it when its time is up. None for a critical one.
    timer: Option<slint::Timer>,
    /// Where it belongs, and where the pointer was when a drag began.
    home: (i32, i32),
    drag_from: Option<i32>,
    /// Whether its window has had its corners rounded.
    settled: bool,
}

#[derive(Default)]
struct State {
    list: Vec<Notification>,
    settings: Option<ToastSettings>,
    /// Off the screen, still in the history.
    expired: HashSet<u32>,
    shown: Vec<Shown>,
    act: Option<Rc<dyn Fn(Action)>>,
    ages: Option<slint::Timer>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Recolours the banners already made, as the theme goes light or dark.
pub(crate) fn repaint(light: bool) {
    STATE.with(|state| {
        for shown in &state.borrow().shown {
            shown.window.global::<crate::Palette>().set_light(light);
        }
    });
}

/// Hands the banners the closing of one, on the native thread.
pub(crate) fn set_act(act: Rc<dyn Fn(Action)>) {
    STATE.with(|state| state.borrow_mut().act = Some(act));
}

/// The notifications and the settings, as they are now. Native thread only.
pub(crate) fn show(list: &serde_json::Value, settings: ToastSettings) {
    let list: Vec<Notification> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| Notification::deserialize(value).ok())
        .collect();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.list = list;
        state.settings = Some(settings);
    });
    render();
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// How much of a banner's time is left, in milliseconds — `toastLife` in
/// `Toasts.tsx`. Measured from when it was posted, so the history a start finds
/// on disk is already over rather than every past banner at once.
fn life(time: u64, timeout: u32, now_ms: u128) -> i128 {
    i128::from(timeout) - (now_ms as i128 - i128::from(time) * 1000)
}

/// The newest of each application's notifications, and how many more there
/// are behind it, in the order the list has them, newest first.
fn groups(list: &[Notification], expired: &HashSet<u32>) -> Vec<(Notification, usize)> {
    let mut order: Vec<String> = Vec::new();
    let mut by_app: HashMap<String, Vec<&Notification>> = HashMap::new();
    for notification in list.iter().filter(|n| !expired.contains(&n.id)) {
        if !by_app.contains_key(&notification.app_name) {
            order.push(notification.app_name.clone());
        }
        by_app
            .entry(notification.app_name.clone())
            .or_default()
            .push(notification);
    }
    order
        .into_iter()
        .filter_map(|app| {
            let all = by_app.remove(&app)?;
            Some(((*all.first()?).clone(), all.len() - 1))
        })
        .collect()
}

/// Brings the windows in line with the list.
fn render() {
    let Some(settings) = STATE.with(|state| state.borrow().settings.clone()) else {
        return;
    };

    // Already over before it was ever shown: into the history and no further.
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let wanted = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let stale: Vec<u32> = state
            .list
            .iter()
            .filter(|n| n.urgency != "critical" && life(n.time, settings.timeout, now_ms) <= 0)
            .map(|n| n.id)
            .collect();
        state.expired.extend(stale);
        if settings.do_not_disturb {
            return Vec::new();
        }
        let mut wanted = groups(&state.list, &state.expired);
        wanted.truncate(settings.max_visible as usize);
        wanted
    });

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        // One window per banner, kept and reused.
        while state.shown.len() < wanted.len() {
            match make_window() {
                Ok(shown) => state.shown.push(shown),
                Err(error) => {
                    tracing::warn!(%error, "could not make a notification banner");
                    break;
                }
            }
        }
        for (index, shown) in state.shown.iter_mut().enumerate() {
            let Some((notification, more)) = wanted.get(index) else {
                let _ = shown.window.hide();
                shown.timer = None;
                shown.id = 0;
                continue;
            };
            fill(&shown.window, notification, *more, settings.japanese);
            if shown.id != notification.id {
                shown.id = notification.id;
                shown.timer = expiry(notification, &settings, now_ms);
            }
            let _ = shown.window.show();
        }
        if state.ages.is_none() && !wanted.is_empty() {
            let ages = slint::Timer::default();
            ages.start(
                slint::TimerMode::Repeated,
                Duration::from_secs(30),
                refresh_ages,
            );
            state.ages = Some(ages);
        }
    });

    // Once the windows exist and have their heights, which is after the loop
    // has had a turn.
    slint::Timer::single_shot(Duration::ZERO, || {
        settle_new();
        place();
    });
}

/// Rounds the corners of, and keeps out of the foreground, every banner whose
/// window has come to exist since. One that has not yet — a window is only
/// made once the loop gets to it, which can be a turn or two after `show` —
/// is looked at again shortly, rather than left square for good.
fn settle_new() {
    let waiting = STATE.with(|state| {
        let mut waiting = false;
        for shown in state
            .borrow_mut()
            .shown
            .iter_mut()
            .filter(|shown| !shown.settled)
        {
            match window_of(shown.window.window()) {
                Some(hwnd) => {
                    settle(hwnd, None);
                    round_corners(hwnd);
                    shown.settled = true;
                }
                None => waiting = true,
            }
        }
        waiting
    });
    if waiting {
        slint::Timer::single_shot(Duration::from_millis(50), settle_new);
    }
}

fn make_window() -> Result<Shown, slint::PlatformError> {
    let window = Toast::new()?;
    window
        .global::<crate::Palette>()
        .set_light(crate::is_light());
    let weak = window.as_weak();
    window.on_close(move || {
        if let Some(window) = weak.upgrade() {
            dismiss(&window);
        }
    });
    let weak = window.as_weak();
    window.on_drag_start(move || {
        let Some(window) = weak.upgrade() else { return };
        with_shown(&window, |shown| shown.drag_from = Some(cursor_x()));
    });
    let weak = window.as_weak();
    window.on_drag_move(move || {
        let Some(window) = weak.upgrade() else { return };
        with_shown(&window, |shown| {
            if let Some(from) = shown.drag_from {
                let (x, y) = shown.home;
                shown
                    .window
                    .window()
                    .set_position(slint::PhysicalPosition::new(x + cursor_x() - from, y));
            }
        });
    });
    let weak = window.as_weak();
    window.on_drag_end(move || {
        let Some(window) = weak.upgrade() else { return };
        let mut gone = false;
        with_shown(&window, |shown| {
            let Some(from) = shown.drag_from.take() else {
                return;
            };
            let threshold = 100.0 * shown.window.window().scale_factor();
            if ((cursor_x() - from).abs() as f32) > threshold {
                gone = true;
            } else {
                let (x, y) = shown.home;
                shown
                    .window
                    .window()
                    .set_position(slint::PhysicalPosition::new(x, y));
            }
        });
        if gone {
            dismiss(&window);
        }
    });
    Ok(Shown {
        id: 0,
        window,
        timer: None,
        home: (0, 0),
        drag_from: None,
        settled: false,
    })
}

/// Runs `f` on the banner this window is.
fn with_shown(window: &Toast, f: impl FnOnce(&mut Shown)) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(shown) = state
            .shown
            .iter_mut()
            .find(|shown| std::ptr::eq(shown.window.window(), window.window()))
        {
            f(shown);
        }
    });
}

/// Closed or swiped away: off the screen at once, and out of the history
/// through the shell, which sends the list back without it.
fn dismiss(window: &Toast) {
    let mut id = 0;
    with_shown(window, |shown| id = shown.id);
    if id == 0 {
        return;
    }
    let act = STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.expired.insert(id);
        state.act.clone()
    });
    render();
    if let Some(act) = act {
        act(Action::Dismiss(id));
    }
}

/// Hides a banner when its time is up. A critical one waits to be dismissed.
fn expiry(
    notification: &Notification,
    settings: &ToastSettings,
    now_ms: u128,
) -> Option<slint::Timer> {
    if notification.urgency == "critical" {
        return None;
    }
    let left = life(notification.time, settings.timeout, now_ms).max(0) as u64;
    let id = notification.id;
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::SingleShot,
        Duration::from_millis(left),
        move || {
            STATE.with(|state| state.borrow_mut().expired.insert(id));
            render();
        },
    );
    Some(timer)
}

fn fill(window: &Toast, notification: &Notification, more: usize, japanese: bool) {
    let glyph = if notification.image.is_empty() {
        "notifications"
    } else {
        "image"
    };
    window.set_glyph(icon(glyph, 18.0, false));
    window.set_critical(notification.urgency == "critical");
    window.set_app(notification.app_name.as_str().into());
    window.set_summary(translate(&notification.app_name, &notification.summary, japanese).into());
    window.set_body(translate(&notification.app_name, &notification.body, japanese).into());
    window.set_age(age(notification.time, now()).into());
    let more = if more == 0 {
        String::new()
    } else if japanese {
        format!("他 {more} 件")
    } else {
        format!("+{more} more")
    };
    window.set_more(more.into());
}

fn refresh_ages() {
    STATE.with(|state| {
        let state = state.borrow();
        for shown in &state.shown {
            if let Some(notification) = state.list.iter().find(|n| n.id == shown.id) {
                shown.window.set_age(age(notification.time, now()).into());
            }
        }
    });
}

/// Lays the banners out from the configured corner, the newest nearest it.
fn place() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(settings) = state.settings.clone() else {
            return;
        };
        let (screen_width, screen_height) = screen_size();
        let bottom = settings.position.starts_with("bottom");
        let mut edge: Option<i32> = None;
        for shown in state.shown.iter_mut().filter(|shown| shown.id != 0) {
            let scale = shown.window.window().scale_factor();
            let size = shown.window.window().size();
            let (width, height) = (size.width as i32, size.height as i32);
            let margin = (8.0 * scale) as i32;
            let bar = if settings.bar {
                (crate::HEIGHT * scale) as i32
            } else {
                0
            };
            let x = if settings.position.ends_with("left") {
                margin
            } else if settings.position.ends_with("center") {
                (screen_width - width) / 2
            } else {
                screen_width - width - margin
            };
            let y = match (bottom, edge) {
                (false, None) => bar + margin,
                (false, Some(below)) => below + margin,
                (true, None) => screen_height - height - margin,
                (true, Some(above)) => above - margin - height,
            };
            edge = Some(if bottom { y } else { y + height });
            shown.home = (x, y);
            shown
                .window
                .window()
                .set_position(slint::PhysicalPosition::new(x, y));
        }
    });
}

fn cursor_x() -> i32 {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        point.x
    }
    #[cfg(not(windows))]
    0
}

/// "now", "3m", "2h", "1d" — `formatAge` in `lib/format.ts`.
fn age(time: u64, now: u64) -> String {
    let elapsed = now.saturating_sub(time);
    let round = |value: f64| value.round() as u64;
    if elapsed < 45 {
        "now".to_owned()
    } else if elapsed < 3600 {
        format!("{}m", round(elapsed as f64 / 60.0))
    } else if elapsed < 86_400 {
        format!("{}h", round(elapsed as f64 / 3600.0))
    } else {
        format!("{}d", round(elapsed as f64 / 86_400.0))
    }
}

/// The shell's own notices, as the Rust side words them — shared with
/// `i18n/index.ts`, which matches them the same way.
const NOTICES: &str = include_str!("../../shell/src/i18n/notices.json");
const JAPANESE: &str = include_str!("../../shell/src/i18n/locales/ja_JP.json");
/// What a dictionary entry ends with when its translation is the English.
const KEEP: &str = "/*keep*/";

thread_local! {
    static DICTIONARY: (Vec<String>, HashMap<String, String>) = (
        serde_json::from_str(NOTICES).unwrap_or_default(),
        serde_json::from_str(JAPANESE).unwrap_or_default(),
    );
}

/// `trNotice`: a notification's text in Japanese when the shell itself wrote
/// it. Other applications' arrive in their own words and are left alone.
fn translate(app: &str, text: &str, japanese: bool) -> String {
    if app != "beautiful-wallpaper" || !japanese {
        return text.to_owned();
    }
    DICTIONARY.with(|(notices, dictionary)| {
        text.split('\n')
            .map(|line| {
                for notice in notices {
                    if let Some(found) = matches(notice, line) {
                        return looked_up(dictionary, notice, &found);
                    }
                }
                looked_up(dictionary, line, &[])
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn looked_up(dictionary: &HashMap<String, String>, source: &str, args: &[String]) -> String {
    let translated = dictionary.get(source).map_or(source, String::as_str);
    let mut text = translated
        .strip_suffix(KEEP)
        .unwrap_or(translated)
        .to_owned();
    for (index, value) in args.iter().enumerate() {
        text = text.replace(&format!("%{}", index + 1), value);
    }
    text
}

/// What a notice's `%1`, `%2`… stand for in `line`, if `line` is that notice.
/// The literal pieces between them are matched in order, each placeholder
/// taking at least one character.
fn matches(notice: &str, line: &str) -> Option<Vec<String>> {
    let mut pieces: Vec<&str> = Vec::new();
    let mut rest = notice;
    while let Some(at) = rest.find('%') {
        let digit = rest[at + 1..].chars().next().filter(char::is_ascii_digit);
        if digit.is_none() {
            break;
        }
        pieces.push(&rest[..at]);
        rest = &rest[at + 2..];
    }
    pieces.push(rest);
    if pieces.len() == 1 {
        return (notice == line).then(Vec::new);
    }

    let (first, last) = (pieces[0], pieces[pieces.len() - 1]);
    let inner = line.strip_prefix(first)?.strip_suffix(last)?;
    let mut found = Vec::new();
    let mut remaining = inner;
    for piece in &pieces[1..pieces.len() - 1] {
        // The placeholder before this piece takes at least one character.
        let start = remaining
            .char_indices()
            .nth(1)
            .map_or(remaining.len(), |(at, _)| at);
        let at = remaining.get(start..)?.find(piece)? + start;
        found.push(remaining[..at].to_owned());
        remaining = &remaining[at + piece.len()..];
    }
    if remaining.is_empty() {
        return None;
    }
    found.push(remaining.to_owned());
    Some(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(id: u32, app: &str) -> Notification {
        Notification {
            id,
            app_name: app.to_owned(),
            summary: String::new(),
            body: String::new(),
            image: String::new(),
            urgency: "normal".to_owned(),
            time: 0,
        }
    }

    #[test]
    fn the_shells_own_notices_are_translated_with_what_fills_them() {
        let text = translate("beautiful-wallpaper", "Not a setting: bar.botom", true);
        assert!(text.contains("bar.botom"), "{text}");
        assert_ne!(text, "Not a setting: bar.botom");
        assert_eq!(
            matches(
                "%1 is not a usable path: %2",
                "C:\\x is not a usable path: gone"
            ),
            Some(vec!["C:\\x".to_owned(), "gone".to_owned()])
        );
    }

    #[test]
    fn other_applications_and_unknown_lines_are_left_alone() {
        assert_eq!(
            translate("Mail", "Not a setting: x", true),
            "Not a setting: x"
        );
        assert_eq!(
            translate("beautiful-wallpaper", "zzz unknown", true),
            "zzz unknown"
        );
        assert_eq!(
            translate("beautiful-wallpaper", "Not a setting: x", false),
            "Not a setting: x"
        );
    }

    #[test]
    fn banners_group_by_application_newest_first() {
        let list = [
            notification(3, "A"),
            notification(2, "B"),
            notification(1, "A"),
        ];
        let found = groups(&list, &HashSet::new());
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].0.id, found[0].1), (3, 1));
        assert_eq!((found[1].0.id, found[1].1), (2, 0));
        let gone = HashSet::from([3]);
        assert_eq!(groups(&list, &gone)[0].0.id, 2);
    }

    #[test]
    fn a_banner_posted_long_ago_has_no_time_left() {
        assert!(life(0, 7000, 1_000_000) <= 0);
        assert_eq!(life(10, 7000, 12_000), 5000);
        assert_eq!(age(100, 120), "now");
        assert_eq!(age(0, 600), "10m");
    }
}
