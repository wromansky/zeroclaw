//! Receipt-verified reply assertions: catch a final answer that claims an
//! action the turn never performed.
//!
//! [`tool_receipts`](super::tool_receipts) proves that a tool result the model
//! *cites* really came from the runtime. It cannot help when the model cites
//! nothing at all — it narrates the action in prose and emits no tool call.
//! The per-turn receipt collector is then empty, and that emptiness is the
//! signal these predicates read.
//!
//! Both functions are pure, so the cost when the feature is off is one `bool`
//! check at the single call site in
//! [`run_tool_call_loop`](super::turn::run_tool_call_loop). The collector
//! format they read is produced by `turn::results_collect`, which stores each
//! entry as `"{tool_name}: {receipt}"`.

/// Whether the reply asserts that an action happened, by case-insensitive
/// substring match against `patterns`. Blank patterns are ignored rather than
/// matching everything; an empty list disables the check.
#[must_use]
pub fn asserts_action(response: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let haystack = response.to_lowercase();
    patterns
        .iter()
        .filter(|p| !p.trim().is_empty())
        .any(|p| haystack.contains(&p.to_lowercase()))
}

/// Whether the turn executed a tool that could have performed the asserted
/// action. An empty `write_tools` means any executed tool qualifies.
#[must_use]
pub fn has_qualifying_receipt(receipts: &[String], write_tools: &[String]) -> bool {
    if receipts.is_empty() {
        return false;
    }
    let wanted: Vec<String> = write_tools
        .iter()
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.to_lowercase())
        .collect();
    if wanted.is_empty() {
        return true;
    }
    receipts.iter().any(|entry| {
        let name = receipt_tool_name(entry).to_lowercase();
        wanted.iter().any(|w| name.contains(w))
    })
}

/// Recover the tool name from a `"{tool_name}: {receipt}"` collector entry.
/// An entry without the separator is treated as a bare tool name, so a change
/// to the collector format degrades to "unknown tool" rather than panicking.
fn receipt_tool_name(entry: &str) -> &str {
    entry.split_once(": ").map_or(entry, |(name, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns() -> Vec<String> {
        vec!["logged".to_string(), "✅".to_string()]
    }

    #[test]
    fn a_claim_is_recognised_case_insensitively() {
        assert!(asserts_action("Logged ✅ — set 3: 80 × 9", &patterns()));
        assert!(asserts_action("LOGGED that one.", &patterns()));
    }

    #[test]
    fn a_reply_without_a_claim_is_not_an_assertion() {
        assert!(!asserts_action("Nice work — rest 2 minutes.", &patterns()));
    }

    #[test]
    fn an_empty_pattern_list_disables_the_check() {
        assert!(!asserts_action("Logged ✅", &[]));
    }

    #[test]
    fn blank_patterns_are_ignored_rather_than_matching_everything() {
        assert!(!asserts_action("anything at all", &["   ".to_string()]));
    }

    // An empty reply asserts nothing. The "model shipped nothing at all"
    // failure is a separate class and not this module's business.
    #[test]
    fn an_empty_reply_asserts_nothing() {
        assert!(!asserts_action("", &patterns()));
    }

    #[test]
    fn no_receipts_means_nothing_qualifies() {
        assert!(!has_qualifying_receipt(&[], &[]));
        assert!(!has_qualifying_receipt(&[], &["log_set".to_string()]));
    }

    #[test]
    fn any_receipt_qualifies_when_no_write_tools_are_named() {
        let receipts = vec!["log_set: zc-receipt-1-abc".to_string()];
        assert!(has_qualifying_receipt(&receipts, &[]));
    }

    // The case `write_tools` exists for: a read ran, the reply claims a write.
    #[test]
    fn a_read_only_receipt_does_not_satisfy_a_write_tool_list() {
        let receipts = vec!["get_workout_for_date: zc-receipt-1-abc".to_string()];
        assert!(!has_qualifying_receipt(&receipts, &["log_set".to_string()]));
    }

    #[test]
    fn a_matching_receipt_satisfies_a_write_tool_list() {
        let receipts = vec![
            "get_workout_for_date: zc-receipt-1-abc".to_string(),
            "wger__log_set: zc-receipt-2-def".to_string(),
        ];
        assert!(has_qualifying_receipt(&receipts, &["log_set".to_string()]));
    }

    #[test]
    fn blank_write_tools_do_not_narrow_the_check() {
        let receipts = vec!["echo: zc-receipt-1-abc".to_string()];
        assert!(has_qualifying_receipt(&receipts, &["  ".to_string()]));
    }

    #[test]
    fn a_receipt_entry_without_the_separator_degrades_to_a_bare_name() {
        assert_eq!(receipt_tool_name("log_set: zc-receipt-1-abc"), "log_set");
        assert_eq!(receipt_tool_name("malformed"), "malformed");
    }
}
