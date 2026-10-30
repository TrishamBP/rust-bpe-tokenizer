// BPE algorithm

use std::fs;
use std::io;
use std::path::PathBuf;

use super::bytes::text_to_token_ids;
use super::decoder::{DecodeError, decode};
use super::encoder::encode;
use super::merger::merge_pair;
use super::pairs::{count_pairs, most_frequent_pair};
use super::vocabulary::{TokenId, Vocabulary};

/// The number of single-byte tokens that every vocabulary starts with.
pub const INITIAL_VOCAB_SIZE: usize = 256;

/// The number of merges between two progress messages.
const PROGRESS_INTERVAL: usize = 25;

/// Settings for one training run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingConfig {
    /// The path of the text file to train on.
    pub corpus_path: PathBuf,
    /// Training stops when the vocabulary has this many tokens.
    pub target_vocab_size: usize,
    /// If set, training stops after this many merges.
    pub max_merges: Option<usize>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        TrainingConfig {
            corpus_path: PathBuf::from("data/shakespeare.txt"),
            target_vocab_size: 512,
            max_merges: None,
        }
    }
}

/// One learned merge: `left` followed by `right` becomes `new_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeRule {
    pub left: TokenId,
    pub right: TokenId,
    pub new_id: TokenId,
    /// How many times the pair occurred when it was selected.
    pub frequency: usize,
}

/// The reason why training stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    TargetVocabSizeReached,
    MaxMergesReached,
    NoPairsRemaining,
}

/// The output of a training run.
pub struct TrainingResult {
    pub vocabulary: Vocabulary,
    /// The merges, in the order in which they were learned.
    pub merges: Vec<MergeRule>,
    /// The number of tokens before the first merge. This is the corpus size in bytes.
    pub initial_token_count: usize,
    /// The training text after all merges.
    pub tokens: Vec<TokenId>,
    pub stop_reason: StopReason,
}

/// A trained tokenizer: its settings, its vocabulary and its ordered merge rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokenizer {
    pub config: TrainingConfig,
    pub vocabulary: Vocabulary,
    pub merges: Vec<MergeRule>,
}

impl Tokenizer {
    /// Converts `text` into token IDs with the learned merge rules.
    pub fn encode(&self, text: &str) -> Vec<TokenId> {
        encode(text, &self.merges)
    }

    /// Converts token IDs back into text with the vocabulary.
    pub fn decode(&self, tokens: &[TokenId]) -> Result<String, DecodeError> {
        decode(tokens, &self.vocabulary)
    }
}

/// Reads the corpus at `config.corpus_path` and trains on it.
/// Returns an error if the file cannot be read as UTF-8 text.
pub fn train_from_file(config: &TrainingConfig) -> io::Result<TrainingResult> {
    let text = fs::read_to_string(&config.corpus_path)?;
    Ok(train(&text, config))
}

/// Learns BPE merges from `text` until a stop condition occurs.
pub fn train(text: &str, config: &TrainingConfig) -> TrainingResult {
    let mut tokens = text_to_token_ids(text);
    let initial_token_count = tokens.len();
    let mut vocabulary = Vocabulary::new();
    let mut merges = Vec::new();

    let stop_reason = loop {
        if vocabulary.len() >= config.target_vocab_size {
            break StopReason::TargetVocabSizeReached;
        }
        if let Some(max_merges) = config.max_merges {
            if merges.len() >= max_merges {
                break StopReason::MaxMergesReached;
            }
        }

        let counts = count_pairs(&tokens);
        let Some((pair, frequency)) = most_frequent_pair(&counts) else {
            break StopReason::NoPairsRemaining;
        };

        // Both IDs come from `tokens`, so the vocabulary always knows them.
        let new_id = vocabulary
            .add_merge(pair)
            .expect("pair IDs must exist in the vocabulary");
        tokens = merge_pair(&tokens, pair, new_id);
        merges.push(MergeRule {
            left: pair.0,
            right: pair.1,
            new_id,
            frequency,
        });

        if merges.len() % PROGRESS_INTERVAL == 0 {
            println!(
                "merge {:4} | vocab size {:4} | pair ({:3}, {:3}) -> {:4} | frequency {:6}",
                merges.len(),
                vocabulary.len(),
                pair.0,
                pair.1,
                new_id,
                frequency
            );
        }
    };

    TrainingResult {
        vocabulary,
        merges,
        initial_token_count,
        tokens,
        stop_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(target_vocab_size: usize, max_merges: Option<usize>) -> TrainingConfig {
        TrainingConfig {
            target_vocab_size,
            max_merges,
            ..TrainingConfig::default()
        }
    }

    fn rule(left: TokenId, right: TokenId, new_id: TokenId, frequency: usize) -> MergeRule {
        MergeRule {
            left,
            right,
            new_id,
            frequency,
        }
    }

    #[test]
    fn target_of_256_learns_nothing() {
        let result = train("banana", &config(INITIAL_VOCAB_SIZE, None));
        assert_eq!(result.vocabulary.len(), INITIAL_VOCAB_SIZE);
        assert!(result.merges.is_empty());
        assert_eq!(result.initial_token_count, 6);
        assert_eq!(result.tokens, vec![98, 97, 110, 97, 110, 97]);
        assert_eq!(result.stop_reason, StopReason::TargetVocabSizeReached);
    }

    #[test]
    fn one_iteration_merges_most_frequent_pair() {
        let result = train("banana", &config(257, None));
        assert_eq!(result.merges, vec![rule(97, 110, 256, 2)]);
        assert_eq!(result.tokens, vec![98, 256, 256, 97]);
        assert_eq!(result.vocabulary.get(256), Some(&b"an"[..]));
    }

    #[test]
    fn multiple_iterations_build_on_earlier_merges() {
        let result = train("banana", &config(259, None));
        assert_eq!(result.tokens, vec![257, 258]);
        assert_eq!(result.vocabulary.get(257), Some(&b"ban"[..]));
        assert_eq!(result.vocabulary.get(258), Some(&b"ana"[..]));
    }

    #[test]
    fn stops_at_target_vocab_size() {
        let result = train("aaaaaaaaaaaa", &config(258, None));
        assert_eq!(result.vocabulary.len(), 258);
        assert_eq!(result.merges.len(), 2);
        assert_eq!(result.stop_reason, StopReason::TargetVocabSizeReached);
    }

    #[test]
    fn stops_at_max_merges() {
        let result = train("banana", &config(512, Some(1)));
        assert_eq!(result.merges.len(), 1);
        assert_eq!(result.vocabulary.len(), 257);
        assert_eq!(result.stop_reason, StopReason::MaxMergesReached);
    }

    #[test]
    fn empty_input_stops_without_merges() {
        let result = train("", &config(512, None));
        assert!(result.tokens.is_empty());
        assert!(result.merges.is_empty());
        assert_eq!(result.stop_reason, StopReason::NoPairsRemaining);
    }

    #[test]
    fn stops_when_no_pairs_remain() {
        // A single byte has no pair.
        let result = train("a", &config(512, None));
        assert!(result.merges.is_empty());
        assert_eq!(result.stop_reason, StopReason::NoPairsRemaining);

        // "banana" becomes one token after 4 merges. Then no pair remains.
        let result = train("banana", &config(512, None));
        assert_eq!(result.merges.len(), 4);
        assert_eq!(result.tokens, vec![259]);
        assert_eq!(result.vocabulary.get(259), Some(&b"banana"[..]));
        assert_eq!(result.stop_reason, StopReason::NoPairsRemaining);
    }

    #[test]
    fn merge_rules_are_stored_in_order() {
        let result = train("banana", &config(512, None));
        assert_eq!(
            result.merges,
            vec![
                rule(97, 110, 256, 2),  // "a" + "n"     -> "an"
                rule(98, 256, 257, 1),  // "b" + "an"    -> "ban"
                rule(256, 97, 258, 1),  // "an" + "a"    -> "ana"
                rule(257, 258, 259, 1), // "ban" + "ana" -> "banana"
            ]
        );
    }

    #[test]
    fn ties_select_smallest_pair_every_time() {
        // All three pairs occur once: (120, 121), (121, 97), (97, 98).
        let first = train("xyab", &config(257, None));
        assert_eq!(first.merges, vec![rule(97, 98, 256, 1)]);

        for _ in 0..10 {
            let again = train("xyab", &config(257, None));
            assert_eq!(again.merges, first.merges);
        }
    }

    #[test]
    fn missing_corpus_file_returns_error() {
        let config = TrainingConfig {
            corpus_path: PathBuf::from("data/this_file_does_not_exist.txt"),
            ..TrainingConfig::default()
        };
        let result = train_from_file(&config);
        assert!(result.is_err());
    }
}
