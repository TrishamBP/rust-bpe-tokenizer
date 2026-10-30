// Text to token IDs

use super::bpe::MergeRule;
use super::bytes::text_to_token_ids;
use super::merger::merge_pair;
use super::vocabulary::TokenId;

/// Converts `text` into token IDs.
/// The function applies each rule in `merges` one time, in the order given.
pub fn encode(text: &str, merges: &[MergeRule]) -> Vec<TokenId> {
    let mut tokens = text_to_token_ids(text);
    for rule in merges {
        tokens = merge_pair(&tokens, (rule.left, rule.right), rule.new_id);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(left: TokenId, right: TokenId, new_id: TokenId) -> MergeRule {
        MergeRule {
            left,
            right,
            new_id,
            frequency: 0,
        }
    }

    /// The example rules: "he" -> 256, "ll" -> 257, "he" + "ll" -> 258.
    fn hello_rules() -> Vec<MergeRule> {
        vec![
            rule(104, 101, 256),
            rule(108, 108, 257),
            rule(256, 257, 258),
        ]
    }

    #[test]
    fn encodes_simple_ascii_word() {
        assert_eq!(encode("hello", &hello_rules()), vec![258, 111]);
    }

    #[test]
    fn without_rules_each_byte_is_a_token() {
        assert_eq!(encode("hello", &[]), vec![104, 101, 108, 108, 111]);
    }

    #[test]
    fn encodes_repeated_characters() {
        // "aaaaa": "aa" -> 256, then "aa" + "aa" -> 257.
        let rules = vec![rule(97, 97, 256), rule(256, 256, 257)];
        assert_eq!(encode("aaaaa", &rules), vec![257, 97]);
    }

    #[test]
    fn encodes_empty_text() {
        assert!(encode("", &hello_rules()).is_empty());
    }

    #[test]
    fn rule_order_changes_the_result() {
        // "abc" with "ab" first gives [ab, c]. With "bc" first, it gives [a, bc].
        let ab_first = vec![rule(97, 98, 256), rule(98, 99, 257)];
        let bc_first = vec![rule(98, 99, 256), rule(97, 98, 257)];
        assert_eq!(encode("abc", &ab_first), vec![256, 99]);
        assert_eq!(encode("abc", &bc_first), vec![97, 256]);
    }
}
