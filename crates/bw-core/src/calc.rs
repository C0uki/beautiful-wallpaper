//! The launcher's calculator.
//!
//! The original hands the query to `qalc` (libqalculate) and shows whatever
//! comes back. There is no `qalc` on Windows and no package manager to fetch
//! one from, so this is a small evaluator instead: arithmetic, parentheses, a
//! handful of functions and two constants. It is deliberately less than
//! libqalculate — no units, no currencies, no symbolic algebra — and says so
//! by returning `None` rather than guessing.
//!
//! The part that matters for the launcher is knowing when *not* to answer.
//! Every keystroke goes through here, so anything that reads as a program
//! name, a path or a search phrase has to come back as "not an expression" —
//! otherwise a calculator row appears above the application the user is
//! actually trying to open.

/// Whether a calculator row belongs in the results for this query.
///
/// Requires both a digit and something to do with it. `notepad` has no digit,
/// `1080p` has no operator, and neither should push a result off the list.
pub fn looks_like_expression(input: &str) -> bool {
    let trimmed = input.trim();
    if !trimmed.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    trimmed
        .chars()
        .any(|ch| matches!(ch, '+' | '-' | '*' | '/' | '%' | '^' | '('))
}

/// Evaluates an expression, or `None` if it is not one this understands.
///
/// A result that is not a finite number — a division by zero, the square root
/// of a negative — is also `None`: showing `inf` or `NaN` in a launcher tells
/// the user nothing they can use.
pub fn evaluate(input: &str) -> Option<f64> {
    let tokens = tokenize(input)?;
    let rpn = to_postfix(&tokens)?;
    let value = evaluate_postfix(&rpn)?;
    value.is_finite().then_some(value)
}

/// Formats a result the way the launcher shows it.
///
/// Trailing zeroes are noise — `2 + 2` should read `4`, not `4.000000` — but
/// a genuine fraction keeps enough digits to be worth having.
pub fn format(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        return (value as i64).to_string();
    }
    let rendered = format!("{value:.10}");
    let trimmed = rendered.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_owned()
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    /// A bare word: a function if a `(` follows it, otherwise a constant.
    Name(String),
    Operator(char),
    Open,
    Close,
}

fn tokenize(input: &str) -> Option<Vec<Token>> {
    let characters: Vec<char> = input.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut index = 0;

    while index < characters.len() {
        let ch = characters[index];

        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        if ch.is_ascii_digit() || ch == '.' {
            let start = index;
            let mut seen_point = false;
            while index < characters.len() {
                let digit = characters[index];
                if digit.is_ascii_digit() {
                    index += 1;
                } else if digit == '.' && !seen_point {
                    seen_point = true;
                    index += 1;
                } else {
                    break;
                }
            }
            let text: String = characters[start..index].iter().collect();
            tokens.push(Token::Number(text.parse().ok()?));
            continue;
        }

        if ch.is_alphabetic() {
            let start = index;
            while index < characters.len() && characters[index].is_alphanumeric() {
                index += 1;
            }
            let name: String = characters[start..index]
                .iter()
                .collect::<String>()
                .to_lowercase();
            tokens.push(Token::Name(name));
            continue;
        }

        index += 1;
        match ch {
            '(' => tokens.push(Token::Open),
            ')' => tokens.push(Token::Close),
            '+' | '*' | '/' | '%' | '^' => tokens.push(Token::Operator(ch)),
            '-' => {
                // A minus is unary when there is no value to its left. Kept as
                // its own operator so that `-2^2` binds the way arithmetic
                // says it does — negate the power, not the base.
                let unary = match tokens.last() {
                    None | Some(Token::Operator(_)) | Some(Token::Open) => true,
                    Some(Token::Name(name)) => is_function(name),
                    _ => false,
                };
                tokens.push(Token::Operator(if unary { NEGATE } else { '-' }));
            }
            _ => return None,
        }
    }

    (!tokens.is_empty()).then_some(tokens)
}

/// Unary minus, kept apart from subtraction inside the evaluator.
const NEGATE: char = '~';

fn is_function(name: &str) -> bool {
    matches!(
        name,
        "sqrt" | "abs" | "sin" | "cos" | "tan" | "log" | "ln" | "round" | "floor" | "ceil"
    )
}

fn constant(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        "tau" => Some(std::f64::consts::TAU),
        _ => None,
    }
}

fn precedence(operator: char) -> u8 {
    match operator {
        '+' | '-' => 1,
        '*' | '/' | '%' => 2,
        NEGATE => 3,
        '^' => 4,
        _ => 0,
    }
}

fn is_right_associative(operator: char) -> bool {
    matches!(operator, '^' | NEGATE)
}

/// Shunting-yard, with functions riding the operator stack.
fn to_postfix(tokens: &[Token]) -> Option<Vec<Token>> {
    let mut output: Vec<Token> = Vec::new();
    let mut stack: Vec<Token> = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Number(_) => output.push(token.clone()),
            Token::Name(name) => {
                if matches!(tokens.get(index + 1), Some(Token::Open)) {
                    if !is_function(name) {
                        return None;
                    }
                    stack.push(token.clone());
                } else {
                    output.push(Token::Number(constant(name)?));
                }
            }
            Token::Operator(operator) => {
                while let Some(&Token::Operator(top)) = stack.last() {
                    let outranked = precedence(top) > precedence(*operator)
                        || (precedence(top) == precedence(*operator)
                            && !is_right_associative(*operator));
                    if !outranked {
                        break;
                    }
                    output.push(stack.pop()?);
                }
                stack.push(token.clone());
            }
            Token::Open => stack.push(Token::Open),
            Token::Close => {
                loop {
                    match stack.pop() {
                        Some(Token::Open) => break,
                        // An unmatched `)` — the user is mid-edit, not asking
                        // for an answer.
                        None => return None,
                        Some(other) => output.push(other),
                    }
                }
                if matches!(stack.last(), Some(Token::Name(_))) {
                    output.push(stack.pop()?);
                }
            }
        }
    }

    while let Some(token) = stack.pop() {
        if matches!(token, Token::Open) {
            return None;
        }
        output.push(token);
    }
    Some(output)
}

fn evaluate_postfix(rpn: &[Token]) -> Option<f64> {
    let mut stack: Vec<f64> = Vec::new();

    for token in rpn {
        match token {
            Token::Number(value) => stack.push(*value),
            Token::Operator(NEGATE) => {
                let value = stack.pop()?;
                stack.push(-value);
            }
            Token::Operator(operator) => {
                let right = stack.pop()?;
                let left = stack.pop()?;
                stack.push(match operator {
                    '+' => left + right,
                    '-' => left - right,
                    '*' => left * right,
                    '/' => left / right,
                    '%' => left % right,
                    '^' => left.powf(right),
                    _ => return None,
                });
            }
            Token::Name(name) => {
                let value = stack.pop()?;
                stack.push(match name.as_str() {
                    "sqrt" => value.sqrt(),
                    "abs" => value.abs(),
                    "sin" => value.sin(),
                    "cos" => value.cos(),
                    "tan" => value.tan(),
                    "log" => value.log10(),
                    "ln" => value.ln(),
                    "round" => value.round(),
                    "floor" => value.floor(),
                    "ceil" => value.ceil(),
                    _ => return None,
                });
            }
            // Parentheses never survive the conversion.
            Token::Open | Token::Close => return None,
        }
    }

    // Exactly one value left, or the expression was `1 2` and means nothing.
    match stack.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(input: &str) -> f64 {
        evaluate(input).unwrap_or_else(|| panic!("{input} should evaluate"))
    }

    #[test]
    fn arithmetic_follows_precedence() {
        assert_eq!(value("2 + 2"), 4.0);
        assert_eq!(value("2 + 3 * 4"), 14.0);
        assert_eq!(value("(2 + 3) * 4"), 20.0);
        assert_eq!(value("10 / 4"), 2.5);
        assert_eq!(value("10 % 3"), 1.0);
    }

    #[test]
    fn powers_are_right_associative() {
        assert_eq!(value("2 ^ 3 ^ 2"), 512.0);
    }

    /// `-2^2` is minus four everywhere else, and has to be here too.
    #[test]
    fn unary_minus_binds_looser_than_a_power() {
        assert_eq!(value("-2 ^ 2"), -4.0);
        assert_eq!(value("-3 + 5"), 2.0);
        assert_eq!(value("2 * -3"), -6.0);
        assert_eq!(value("sqrt(-0 + 4)"), 2.0);
    }

    #[test]
    fn functions_and_constants_work() {
        assert_eq!(value("sqrt(16)"), 4.0);
        assert_eq!(value("floor(3.7)"), 3.0);
        assert_eq!(value("abs(0 - 5)"), 5.0);
        assert!((value("2 * pi") - std::f64::consts::TAU).abs() < 1e-12);
    }

    #[test]
    fn an_unfinished_expression_is_not_an_answer() {
        assert!(evaluate("1 +").is_none());
        assert!(evaluate("(1 + 2").is_none());
        assert!(evaluate("1 + 2)").is_none());
        assert!(evaluate("*").is_none());
        assert!(evaluate("").is_none());
    }

    #[test]
    fn an_unknown_word_is_not_an_answer() {
        assert!(evaluate("notepad").is_none());
        assert!(evaluate("2 + banana").is_none());
        assert!(evaluate("banana(2)").is_none());
    }

    #[test]
    fn a_result_that_is_not_a_finite_number_is_withheld() {
        assert!(evaluate("1 / 0").is_none());
        assert!(evaluate("sqrt(0 - 1)").is_none());
    }

    #[test]
    fn two_values_with_nothing_joining_them_is_not_an_answer() {
        assert!(evaluate("1 2").is_none());
    }

    /// Every keystroke in the launcher lands here, so nothing may panic.
    #[test]
    fn junk_input_returns_none_rather_than_panicking() {
        for input in [
            "((((",
            "))))",
            "^^^",
            "1..2",
            "--",
            "()",
            "sqrt()",
            "1 + + 2",
            "e e",
            ".",
            "1e999",
            "%%",
            "~",
            "C:\\Program Files\\app.exe",
            "https://example.com/?a=1",
        ] {
            let _ = evaluate(input);
        }
    }

    #[test]
    fn a_query_needs_both_a_number_and_an_operator_to_look_like_arithmetic() {
        assert!(looks_like_expression("2 + 2"));
        assert!(looks_like_expression("sqrt(16)"));
        assert!(!looks_like_expression("notepad"));
        assert!(!looks_like_expression("1080p"));
        assert!(!looks_like_expression("3.14"));
        assert!(!looks_like_expression(""));
    }

    #[test]
    fn whole_results_are_shown_without_a_decimal_tail() {
        assert_eq!(format(4.0), "4");
        assert_eq!(format(-4.0), "-4");
        assert_eq!(format(2.5), "2.5");
        assert_eq!(format(1.0 / 3.0), "0.3333333333");
    }
}
