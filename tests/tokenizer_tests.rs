// Integration tests for rust_tokenizer

use rust_tokenizer::tokenizer::bpe::{TrainingConfig, TrainingResult, train};
use rust_tokenizer::tokenizer::decoder::decode;
use rust_tokenizer::tokenizer::encoder::encode;

/// A short excerpt from the start of the Tiny Shakespeare corpus.
const EXCERPT: &str = "First Citizen:
Before we proceed any further, hear me speak.

All:
Speak, speak.

First Citizen:
You are all resolved rather to die than to famish?

All:
Resolved. resolved.
";

fn train_on_excerpt() -> TrainingResult {
    let config = TrainingConfig {
        target_vocab_size: 300,
        ..TrainingConfig::default()
    };
    train(EXCERPT, &config)
}

#[test]
fn encoding_the_training_text_matches_training_output() {
    let result = train_on_excerpt();
    assert_eq!(encode(EXCERPT, &result.merges), result.tokens);
}

#[test]
fn shakespeare_excerpt_round_trips() {
    let result = train_on_excerpt();
    let encoded = encode(EXCERPT, &result.merges);
    assert!(
        encoded.len() < EXCERPT.len(),
        "the merges must compress the text"
    );
    assert_eq!(
        decode(&encoded, &result.vocabulary),
        Ok(EXCERPT.to_string())
    );
}

#[test]
fn unseen_text_round_trips_exactly() {
    let result = train_on_excerpt();
    for text in [
        "To be, or not to be, that is the question.",
        "  leading and trailing spaces  ",
        "tabs\tand\r\nwindows line endings",
        "café, naïve, 日本語, 🦀",
        "",
    ] {
        let encoded = encode(text, &result.merges);
        assert_eq!(decode(&encoded, &result.vocabulary), Ok(text.to_string()));
    }
}
