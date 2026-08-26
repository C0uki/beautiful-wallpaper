//! Fuzzy matching for the launcher.
//!
//! A launcher lives or dies on whether typing `vsc` finds Visual Studio Code.
//! Getting that right is not a matter of taste: a greedy left-to-right scan
//! matches the `s` inside `Visual` rather than the one starting `Studio`, and
//! then the whole ranking is built on a match nobody would have chosen. So the
//! best alignment is searched for properly rather than taken from the first
//! pass, and the positions it settles on are handed back for the UI to
//! highlight — a result whose highlights disagree with why it ranked where it
//! did looks broken even when the order is right.
//!
//! All of this is pure data so that it is covered by tests that run on Linux.

/// Landing on the first character of the candidate.
const BONUS_START: i32 = 16;
/// Landing just after a separator — the start of a word.
const BONUS_SEPARATOR: i32 = 10;
/// Landing on a case or digit boundary, as in `VSCode` or `Office365`.
const BONUS_BOUNDARY: i32 = 8;
/// Landing immediately after the previous match.
const BONUS_CONSECUTIVE: i32 = 8;
/// Matching the case the user typed.
const BONUS_EXACT_CASE: i32 = 2;
/// Per character skipped between two matches.
const PENALTY_GAP: i32 = 2;
/// Per character skipped before the first match, up to [`LEADING_CAP`].
///
/// The cap matters more than the rate. Uncapped, a match on the last word of
/// a long name pays more than it can ever earn back, and `code` ranks
/// `Barcode Scanner` above `Visual Studio Code`.
const PENALTY_LEADING: i32 = 1;
const LEADING_CAP: i32 = 6;

/// Candidates longer than this are truncated before matching.
///
/// The alignment search is quadratic in how often a character repeats, which
/// is harmless for an application name and unbounded for, say, a pasted path.
const MAX_CANDIDATE: usize = 160;

/// Where a query matched, and how well.
///
/// Rust-only: the frontend never sees one of these. What it needs — the
/// matched positions — is copied onto the result rows themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub score: i32,
    /// Indices of the matched characters, ascending.
    ///
    /// These count *characters*, not bytes and not UTF-16 units, so the
    /// frontend has to index with `Array.from(name)` rather than `name[i]`.
    pub positions: Vec<usize>,
}

/// Scores one candidate, or `None` if the query is not a subsequence of it.
///
/// An empty query matches everything with a score of zero, which is what lets
/// the launcher show a list before anything is typed.
pub fn score(query: &str, candidate: &str) -> Option<Match> {
    // Spaces in a launcher query are how people separate words they expect to
    // be matched loosely, not characters they expect to find.
    let needle: Vec<char> = query.chars().filter(|ch| !ch.is_whitespace()).collect();
    if needle.is_empty() {
        return Some(Match {
            score: 0,
            positions: Vec::new(),
        });
    }

    let haystack: Vec<char> = candidate.chars().take(MAX_CANDIDATE).collect();
    if needle.len() > haystack.len() {
        return None;
    }

    let lowered: Vec<char> = haystack.iter().copied().map(lower).collect();

    // Where each query character could go. Building this first turns the
    // alignment search from "every position against every position" into
    // "every occurrence against every occurrence", and a letter occurs two or
    // three times in a name rather than a hundred.
    let mut rows: Vec<Vec<usize>> = Vec::with_capacity(needle.len());
    for ch in &needle {
        let wanted = lower(*ch);
        let row: Vec<usize> = lowered
            .iter()
            .enumerate()
            .filter(|(_, candidate)| **candidate == wanted)
            .map(|(index, _)| index)
            .collect();
        if row.is_empty() {
            return None;
        }
        rows.push(row);
    }

    // `scores[i][slot]` is the best total for matching the query up to `i`
    // with `needle[i]` landing on `rows[i][slot]`; `parents` remembers which
    // slot of the previous row that total came through, so the winning
    // alignment can be walked back out at the end.
    let mut scores: Vec<Vec<i32>> = Vec::with_capacity(needle.len());
    let mut parents: Vec<Vec<usize>> = Vec::with_capacity(needle.len());

    for (i, row) in rows.iter().enumerate() {
        let mut row_scores = Vec::with_capacity(row.len());
        let mut row_parents = Vec::with_capacity(row.len());

        for &index in row {
            let bonus = bonus_at(&haystack, index)
                + if haystack[index] == needle[i] {
                    BONUS_EXACT_CASE
                } else {
                    0
                };

            if i == 0 {
                let leading = (index as i32).min(LEADING_CAP) * PENALTY_LEADING;
                row_scores.push(bonus - leading);
                row_parents.push(usize::MAX);
                continue;
            }

            let mut best = i32::MIN;
            let mut best_slot = usize::MAX;
            for (slot, &previous_index) in rows[i - 1].iter().enumerate() {
                // Rows are ascending, so once the previous character would sit
                // at or after this one there is nothing left to consider.
                if previous_index >= index {
                    break;
                }
                let previous = scores[i - 1][slot];
                if previous == i32::MIN {
                    continue;
                }
                let gap = (index - previous_index - 1) as i32;
                let total =
                    previous - gap * PENALTY_GAP + if gap == 0 { BONUS_CONSECUTIVE } else { 0 };
                if total > best {
                    best = total;
                    best_slot = slot;
                }
            }

            if best == i32::MIN {
                row_scores.push(i32::MIN);
                row_parents.push(usize::MAX);
            } else {
                row_scores.push(best + bonus);
                row_parents.push(best_slot);
            }
        }

        if row_scores.iter().all(|total| *total == i32::MIN) {
            return None;
        }
        scores.push(row_scores);
        parents.push(row_parents);
    }

    let last = needle.len() - 1;
    let mut best = i32::MIN;
    let mut best_slot = usize::MAX;
    for (slot, total) in scores[last].iter().enumerate() {
        if *total > best {
            best = *total;
            best_slot = slot;
        }
    }
    if best == i32::MIN {
        return None;
    }

    let mut positions = vec![0usize; needle.len()];
    let mut slot = best_slot;
    for i in (0..needle.len()).rev() {
        positions[i] = rows[i][slot];
        if i > 0 {
            slot = parents[i][slot];
        }
    }

    // A short name that matches as well as a long one is the one the user
    // meant: `Code` should beat `Visual Studio Code` for `code`.
    let length_penalty = (haystack.len() as i32).min(64) / 8;
    Some(Match {
        score: best - length_penalty,
        positions,
    })
}

/// Scores every candidate, drops the misses and sorts the rest best first.
///
/// Returns each survivor's index into the input, so the caller keeps whatever
/// it was that the names came from.
pub fn rank<'a>(query: &str, candidates: impl IntoIterator<Item = &'a str>) -> Vec<(usize, Match)> {
    let mut ranked: Vec<(usize, Match)> = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, candidate)| Some((index, score(query, candidate)?)))
        .collect();

    // Ties keep the order they came in, so a list with no query — every score
    // zero — is left exactly as the caller assembled it.
    ranked.sort_by(|(left_index, left), (right_index, right)| {
        right
            .score
            .cmp(&left.score)
            .then(left_index.cmp(right_index))
    });
    ranked
}

/// A one-to-one lowercase, so that positions still index the original.
///
/// `char::to_lowercase` yields a sequence — 'İ' becomes two characters — and
/// using it here would slide every later index along by one.
fn lower(ch: char) -> char {
    ch.to_lowercase().next().unwrap_or(ch)
}

/// What landing on this character is worth, before any gap penalty.
fn bonus_at(candidate: &[char], index: usize) -> i32 {
    if index == 0 {
        return BONUS_START;
    }
    let previous = candidate[index - 1];
    let current = candidate[index];

    if is_separator(previous) {
        BONUS_SEPARATOR
    } else if is_boundary(previous, current) {
        BONUS_BOUNDARY
    } else {
        0
    }
}

/// A word start with nothing separating it: `VSCode`, `Office365`.
fn is_boundary(previous: char, current: char) -> bool {
    (previous.is_lowercase() && current.is_uppercase())
        || previous.is_numeric() != current.is_numeric()
}

fn is_separator(ch: char) -> bool {
    matches!(
        ch,
        ' ' | '-' | '_' | '.' | '/' | '\\' | '(' | ')' | '[' | ']' | ':' | ',' | '\''
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason this module is not a one-line `contains` call.
    #[test]
    fn an_acronym_matches_the_word_starts_rather_than_the_first_letters_it_finds() {
        let found = score("vsc", "Visual Studio Code").expect("an acronym should match");
        assert_eq!(found.positions, vec![0, 7, 14]);
    }

    #[test]
    fn a_word_start_beats_the_same_letters_mid_word() {
        let start = score("code", "Visual Studio Code").expect("matches");
        let middle = score("code", "Barcode Scanner").expect("matches");
        assert!(
            start.score > middle.score,
            "word start {} should beat mid-word {}",
            start.score,
            middle.score
        );
    }

    #[test]
    fn a_shorter_name_wins_an_otherwise_equal_match() {
        let ranked = rank("code", ["Visual Studio Code", "Code"]);
        assert_eq!(ranked[0].0, 1, "the shorter name should come first");
    }

    #[test]
    fn matching_is_case_insensitive_but_the_case_typed_is_rewarded() {
        let exact = score("VS", "VS Code").expect("matches");
        let loose = score("vs", "VS Code").expect("matches");
        assert!(exact.score > loose.score);
    }

    #[test]
    fn camel_case_and_digit_boundaries_count_as_word_starts() {
        let camel = score("vsc", "VSCode").expect("matches");
        assert_eq!(camel.positions, vec![0, 1, 2]);

        let digits = score("o365", "Office365").expect("matches");
        assert_eq!(digits.positions, vec![0, 6, 7, 8]);
    }

    #[test]
    fn spaces_in_the_query_are_separators_rather_than_characters_to_find() {
        let spaced = score("visual code", "Visual Studio Code").expect("matches");
        let tight = score("visualcode", "Visual Studio Code").expect("matches");
        assert_eq!(spaced.positions, tight.positions);
    }

    #[test]
    fn a_query_that_is_not_a_subsequence_does_not_match() {
        assert!(score("xyz", "Visual Studio Code").is_none());
        assert!(score("codes", "Code").is_none());
    }

    #[test]
    fn an_empty_query_matches_everything_in_the_order_it_was_given() {
        let ranked = rank("", ["Second", "First", "Third"]);
        assert_eq!(
            ranked.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(ranked.iter().all(|(_, found)| found.positions.is_empty()));
    }

    #[test]
    fn consecutive_characters_beat_the_same_characters_spread_out() {
        let together = score("abc", "abcdef").expect("matches");
        let apart = score("abc", "axbxcx").expect("matches");
        assert!(together.score > apart.score);
    }

    #[test]
    fn positions_are_ascending_and_index_the_characters_they_matched() {
        let found = score("stud", "Visual Studio Code").expect("matches");
        assert!(found.positions.windows(2).all(|pair| pair[0] < pair[1]));

        let characters: Vec<char> = "Visual Studio Code".chars().collect();
        let matched: String = found
            .positions
            .iter()
            .map(|index| characters[*index])
            .collect();
        assert_eq!(matched.to_lowercase(), "stud");
    }

    /// Names are not all ASCII, and a multi-byte one must not shift the
    /// highlights of everything after it.
    #[test]
    fn positions_count_characters_rather_than_bytes() {
        let found = score("ノ", "メモ帳ノート").expect("matches");
        assert_eq!(found.positions, vec![3]);
    }

    #[test]
    fn a_candidate_longer_than_the_cap_does_not_panic() {
        let long = "a".repeat(MAX_CANDIDATE * 3);
        assert!(score("aaa", &long).is_some());
        assert!(score("b", &long).is_none());
    }

    #[test]
    fn ranking_drops_the_misses() {
        let ranked = rank("code", ["Code", "Notepad", "VS Code"]);
        assert_eq!(ranked.len(), 2);
        assert!(ranked.iter().all(|(index, _)| *index != 1));
    }
}
