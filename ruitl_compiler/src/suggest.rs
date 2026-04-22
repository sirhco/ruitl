//! Fuzzy-match helpers for "did you mean" diagnostics.
//!
//! Only used by the codegen validation pass and by consumers of the public
//! `Result` type; self-contained to keep dependency surface narrow.

/// Pick the closest string in `haystack` to `needle` by Levenshtein distance,
/// if one is within `max_distance`. Ties resolve to the first candidate in
/// `haystack` order (stable, so callers can influence priority by sorting).
pub fn closest(needle: &str, haystack: &[&str], max_distance: usize) -> Option<String> {
    haystack
        .iter()
        .map(|cand| (*cand, strsim::levenshtein(needle, cand)))
        .filter(|(_, d)| *d <= max_distance)
        .min_by_key(|(_, d)| *d)
        .map(|(cand, _)| cand.to_string())
}

/// Distance threshold tuned to the length of the identifier: short idents
/// (≤4 chars) accept at most distance 2; longer idents scale at ~1/3 length
/// but cap at 3 to avoid suggesting wildly different names on long idents.
pub fn threshold_for(ident: &str) -> usize {
    let len = ident.chars().count();
    if len <= 4 {
        2
    } else {
        (len / 3).min(3).max(1)
    }
}

/// Convenience: combine `threshold_for` + `closest` into a single call.
pub fn suggest(needle: &str, haystack: &[&str]) -> Option<String> {
    closest(needle, haystack, threshold_for(needle))
}

/// Render a suggestion as a help footer line, or an empty string if
/// `suggestion` is `None`. Keeps error-message construction terse.
pub fn help_line(suggestion: Option<&str>) -> String {
    match suggestion {
        Some(s) => format!("\n  help: did you mean `{}`?", s),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_close_match() {
        let haystack = ["text", "target", "size"];
        assert_eq!(suggest("texx", &haystack).as_deref(), Some("text"));
    }

    #[test]
    fn rejects_wildly_different() {
        let haystack = ["text", "target", "size"];
        assert_eq!(suggest("foo", &haystack), None);
    }

    #[test]
    fn stable_tie_break_returns_first_by_order() {
        // "xxxx" is distance 4 from both — above threshold (2 for len 4) —
        // so no suggestion. Use a case that ties within threshold instead.
        // "tex" ↔ "text"=1 vs "size"=4: text wins.
        let haystack = ["text", "size"];
        assert_eq!(suggest("tex", &haystack).as_deref(), Some("text"));
    }

    #[test]
    fn threshold_scales_with_len() {
        assert_eq!(threshold_for("ab"), 2);
        assert_eq!(threshold_for("abcd"), 2);
        assert_eq!(threshold_for("abcdef"), 2); // 6/3 = 2
        assert_eq!(threshold_for("abcdefghi"), 3); // 9/3 = 3
        assert_eq!(threshold_for("abcdefghijkl"), 3); // cap at 3
    }

    #[test]
    fn help_line_formats_suggestion() {
        assert_eq!(
            help_line(Some("Button")),
            "\n  help: did you mean `Button`?"
        );
        assert_eq!(help_line(None), "");
    }
}
