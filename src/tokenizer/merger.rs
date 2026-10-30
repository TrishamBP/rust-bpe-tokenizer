// Merging a selected pair of tokens

use super::vocabulary::TokenId;

/// Replaces each non-overlapping occurrence of `pair` in `tokens` with `new_id`.
/// The scan goes from left to right.
pub fn merge_pair(tokens: &[TokenId], pair: (TokenId, TokenId), new_id: TokenId) -> Vec<TokenId> {
    let mut merged = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() && tokens[i] == pair.0 && tokens[i + 1] == pair.1 {
            merged.push(new_id);
            i += 2;
        } else {
            merged.push(tokens[i]);
            i += 1;
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_banana_example() {
        let tokens = [98, 97, 110, 97, 110, 97];
        assert_eq!(merge_pair(&tokens, (97, 110), 256), vec![98, 256, 256, 97]);
    }

    #[test]
    fn merges_one_occurrence() {
        let tokens = [99, 97, 116];
        assert_eq!(merge_pair(&tokens, (97, 116), 256), vec![99, 256]);
    }

    #[test]
    fn merges_multiple_occurrences() {
        let tokens = [97, 98, 120, 97, 98, 121, 97, 98];
        assert_eq!(
            merge_pair(&tokens, (97, 98), 256),
            vec![256, 120, 256, 121, 256]
        );
    }

    #[test]
    fn preserves_tokens_without_match() {
        let tokens = [104, 101, 108, 108, 111];
        assert_eq!(merge_pair(&tokens, (120, 121), 256), tokens.to_vec());
    }

    #[test]
    fn handles_empty_sequence() {
        assert_eq!(merge_pair(&[], (97, 98), 256), Vec::<TokenId>::new());
    }

    #[test]
    fn overlapping_pairs_merge_left_to_right() {
        // "aaa": the first two bytes merge. The third byte has no partner.
        assert_eq!(merge_pair(&[97, 97, 97], (97, 97), 256), vec![256, 97]);
        // "aaaa": two non-overlapping merges.
        assert_eq!(merge_pair(&[97, 97, 97, 97], (97, 97), 256), vec![256, 256]);
    }

    #[test]
    fn merges_tokens_with_ids_above_255() {
        let tokens = [256, 256, 97];
        assert_eq!(merge_pair(&tokens, (256, 256), 257), vec![257, 97]);
    }
}
