use crate::annotation::{AnchorHealth, Selector};

/// Result of trying to match a TextQuoteSelector against document content.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub health: AnchorHealth,
    /// Byte range in the source where the match was found.
    pub range: Option<std::ops::Range<usize>>,
}

/// Try to match a TextQuoteSelector against document content.
pub fn match_text_quote(content: &str, exact: &str, prefix: Option<&str>, suffix: Option<&str>) -> MatchResult {
    // First try exact match
    if let Some(result) = exact_match(content, exact, prefix, suffix) {
        return result;
    }

    // Try fuzzy match
    if let Some(result) = fuzzy_match(content, exact) {
        return result;
    }

    MatchResult {
        health: AnchorHealth::Orphaned,
        range: None,
    }
}

/// Exact text match with optional prefix/suffix disambiguation.
fn exact_match(content: &str, exact: &str, prefix: Option<&str>, suffix: Option<&str>) -> Option<MatchResult> {
    let mut search_from = 0;
    let mut candidates = Vec::new();

    while let Some(pos) = content[search_from..].find(exact) {
        let abs_pos = search_from + pos;
        candidates.push(abs_pos);
        search_from = abs_pos + 1;
    }

    if candidates.is_empty() {
        return None;
    }

    // If only one match, return it
    if candidates.len() == 1 {
        let start = candidates[0];
        return Some(MatchResult {
            health: AnchorHealth::Live,
            range: Some(start..start + exact.len()),
        });
    }

    // Disambiguate with prefix/suffix
    for &start in &candidates {
        let mut score = 0;
        if let Some(pfx) = prefix {
            if start >= pfx.len() {
                let before = &content[start - pfx.len()..start];
                if before.ends_with(pfx) {
                    score += 1;
                }
            }
        }
        if let Some(sfx) = suffix {
            let end = start + exact.len();
            if end + sfx.len() <= content.len() {
                let after = &content[end..end + sfx.len()];
                if after.starts_with(sfx) {
                    score += 1;
                }
            }
        }
        if score > 0 {
            return Some(MatchResult {
                health: AnchorHealth::Live,
                range: Some(start..start + exact.len()),
            });
        }
    }

    // Multiple matches, no disambiguation — return first
    let start = candidates[0];
    Some(MatchResult {
        health: AnchorHealth::Live,
        range: Some(start..start + exact.len()),
    })
}

/// Simple fuzzy matching using longest common substring.
fn fuzzy_match(content: &str, exact: &str) -> Option<MatchResult> {
    // Only attempt fuzzy match for non-trivial strings
    if exact.len() < 10 {
        return None;
    }

    let threshold = (exact.len() as f64 * 0.8) as usize;

    // Sliding window approach: find the best match location
    let mut best_score = 0;
    let mut best_pos = 0;

    for start in 0..content.len().saturating_sub(exact.len() / 2) {
        let end = (start + exact.len() + exact.len() / 4).min(content.len());
        let window = &content[start..end];
        let score = longest_common_subsequence_len(window, exact);
        if score > best_score {
            best_score = score;
            best_pos = start;
        }
    }

    if best_score >= threshold {
        let end = (best_pos + exact.len() + exact.len() / 4).min(content.len());
        Some(MatchResult {
            health: AnchorHealth::Approximate,
            range: Some(best_pos..end),
        })
    } else {
        None
    }
}

/// Length of the longest common subsequence.
fn longest_common_subsequence_len(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let m = a_bytes.len();
    let n = b_bytes.len();

    // Space-optimized LCS: only keep two rows
    let mut prev = vec![0u16; n + 1];
    let mut curr = vec![0u16; n + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a_bytes[i - 1] == b_bytes[j - 1] {
                curr[j] = prev[j - 1] + 1;
            } else {
                curr[j] = prev[j].max(curr[j - 1]);
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }

    prev[n] as usize
}

/// Check health of a set of selectors against document content.
pub fn check_selectors(content: &str, selectors: &[Selector]) -> AnchorHealth {
    for selector in selectors {
        if let Selector::TextQuoteSelector { exact, prefix, suffix } = selector {
            let result = match_text_quote(
                content,
                exact,
                prefix.as_deref(),
                suffix.as_deref(),
            );
            return result.health;
        }
    }

    // No TextQuoteSelector found — check if there's a position selector
    for selector in selectors {
        if let Selector::TextPositionSelector { start: _, end } = selector {
            if *end <= content.len() {
                return AnchorHealth::Stale; // Position exists but no text to verify
            }
        }
    }

    AnchorHealth::Orphaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_simple() {
        let content = "Hello, world! This is a test.";
        let result = match_text_quote(content, "world", None, None);
        assert_eq!(result.health, AnchorHealth::Live);
        assert_eq!(result.range, Some(7..12));
    }

    #[test]
    fn test_exact_match_with_context() {
        let content = "foo bar baz bar qux";
        let result = match_text_quote(content, "bar", Some("foo "), Some(" baz"));
        assert_eq!(result.health, AnchorHealth::Live);
        assert_eq!(result.range, Some(4..7));
    }

    #[test]
    fn test_no_match() {
        let content = "Hello, world!";
        let result = match_text_quote(content, "xyz not here", None, None);
        assert_eq!(result.health, AnchorHealth::Orphaned);
    }
}
