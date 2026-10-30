// Adjacent token-pair frequency counting

use std::collections::HashMap;

use super::vocabulary::TokenId;

/// Counts how many times each pair of adjacent tokens occurs in `tokens`.
pub fn count_pairs(tokens: &[TokenId]) -> HashMap<(TokenId, TokenId), usize> {
    let mut counts = HashMap::new();
    for window in tokens.windows(2) {
        let pair = (window[0], window[1]);
        *counts.entry(pair).or_insert(0) += 1;
    }
    counts
}

/// Returns the pair with the highest count, and its count.
/// If counts are equal, the smaller pair wins. Returns `None` if `counts` is empty.
pub fn most_frequent_pair(
    counts: &HashMap<(TokenId, TokenId), usize>,
) -> Option<((TokenId, TokenId), usize)> {
    let mut best: Option<((TokenId, TokenId), usize)> = None;
    for (&pair, &count) in counts {
        let is_better = match best {
            None => true,
            Some((best_pair, best_count)) => {
                count > best_count || (count == best_count && pair < best_pair)
            }
        };
        if is_better {
            best = Some((pair, count));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_banana_pairs() {
        let counts = count_pairs(&[98, 97, 110, 97, 110, 97]);
        assert_eq!(counts.len(), 3);
        assert_eq!(counts[&(98, 97)], 1);
        assert_eq!(counts[&(97, 110)], 2);
        assert_eq!(counts[&(110, 97)], 2);
    }

    #[test]
    fn single_byte_has_no_pairs() {
        assert!(count_pairs(&[42]).is_empty());
    }

    #[test]
    fn empty_input_has_no_pairs() {
        assert!(count_pairs(&[]).is_empty());
    }

    #[test]
    fn repeated_bytes_count_overlapping_pairs() {
        // "aaaa" has 3 overlapping windows: aa, aa, aa.
        let counts = count_pairs(&[97, 97, 97, 97]);
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[&(97, 97)], 3);
    }

    #[test]
    fn distinct_pairs_each_count_once() {
        // "abcd" has pairs ab, bc, cd.
        let counts = count_pairs(&[97, 98, 99, 100]);
        assert_eq!(counts.len(), 3);
        assert!(counts.values().all(|&count| count == 1));
    }

    #[test]
    fn selects_most_frequent_pair() {
        let counts = count_pairs(&[98, 97, 110, 97, 110, 97, 110]);
        // (97, 110) occurs 3 times; (110, 97) occurs 2 times; (98, 97) occurs 1 time.
        assert_eq!(most_frequent_pair(&counts), Some(((97, 110), 3)));
    }

    #[test]
    fn tie_selects_smaller_pair() {
        // Banana: (97, 110) and (110, 97) both occur 2 times.
        let counts = count_pairs(&[98, 97, 110, 97, 110, 97]);
        assert_eq!(most_frequent_pair(&counts), Some(((97, 110), 2)));
    }

    #[test]
    fn tie_on_first_byte_compares_second_byte() {
        let mut counts = HashMap::new();
        counts.insert((97, 99), 5);
        counts.insert((97, 98), 5);
        counts.insert((98, 97), 5);
        assert_eq!(most_frequent_pair(&counts), Some(((97, 98), 5)));
    }

    #[test]
    fn empty_map_selects_none() {
        let counts = HashMap::new();
        assert_eq!(most_frequent_pair(&counts), None);
    }
}
