use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use rust_tokenizer::tokenizer::bpe::{
    INITIAL_VOCAB_SIZE, Tokenizer, TrainingConfig, train_from_file,
};
use rust_tokenizer::tokenizer::serialization::{DEFAULT_MODEL_PATH, save};

/// The sentence that the program encodes and decodes after training.
const DEMO_TEXT: &str = "To be, or not to be, that is the question.";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = TrainingConfig::default();

    println!("Training corpus:         {}", config.corpus_path.display());
    println!("Initial vocabulary size: {INITIAL_VOCAB_SIZE}");
    println!("Target vocabulary size:  {}", config.target_vocab_size);
    println!();

    let result = train_from_file(&config)?;

    println!();
    println!("Training complete.");
    println!("Stop reason:           {:?}", result.stop_reason);
    println!("Merges completed:      {}", result.merges.len());
    println!("Final vocabulary size: {}", result.vocabulary.len());
    println!("Corpus size (bytes):   {}", result.initial_token_count);
    println!("Final token count:     {}", result.tokens.len());
    println!();

    println!("First 20 merge rules:");
    for (i, rule) in result.merges.iter().take(20).enumerate() {
        let bytes = result.vocabulary.get(rule.new_id).unwrap_or_default();
        println!(
            "{:2}: ({:3}, {:3}) -> {} {:?}  frequency {}",
            i + 1,
            rule.left,
            rule.right,
            rule.new_id,
            String::from_utf8_lossy(bytes),
            rule.frequency
        );
    }
    println!();

    let tokenizer = Tokenizer {
        config,
        vocabulary: result.vocabulary,
        merges: result.merges,
    };

    let model_path = Path::new(DEFAULT_MODEL_PATH);
    save(&tokenizer, model_path)?;
    println!("Saved tokenizer to:    {}", model_path.display());
    println!();

    let encoded = tokenizer.encode(DEMO_TEXT);
    let decoded = tokenizer.decode(&encoded)?;
    println!("Original text:   {DEMO_TEXT:?}");
    println!("Character count: {}", DEMO_TEXT.chars().count());
    println!("Byte count:      {}", DEMO_TEXT.len());
    println!("Encoded IDs:     {encoded:?}");
    println!("Token count:     {}", encoded.len());
    println!("Decoded text:    {decoded:?}");
    println!(
        "Round trip:      {}",
        if decoded == DEMO_TEXT { "OK" } else { "FAILED" }
    );

    Ok(())
}
