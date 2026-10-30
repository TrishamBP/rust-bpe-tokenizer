//! Loads the saved tokenizer and encodes and decodes one sentence.
//! It does not train. Run `cargo run --release` first to create the model file.
//!
//! Usage: cargo run --release --example basic_encoding -- "optional text"

use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use rust_tokenizer::tokenizer::serialization::{DEFAULT_MODEL_PATH, load};

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Returns `Ok(true)` if the decoded text is equal to the input text.
fn run() -> Result<bool, Box<dyn Error>> {
    let text = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Now is the winter of our discontent.".to_string());

    let tokenizer = load(Path::new(DEFAULT_MODEL_PATH))?;
    println!("Loaded tokenizer from: {DEFAULT_MODEL_PATH}");
    println!("Vocabulary size:       {}", tokenizer.vocabulary.len());
    println!("Merge rules:           {}", tokenizer.merges.len());
    println!();

    let encoded = tokenizer.encode(&text);
    let decoded = tokenizer.decode(&encoded)?;
    let matches = decoded == text;

    println!("Original text: {text:?}");
    println!("Byte count:    {}", text.len());
    println!("Encoded IDs:   {encoded:?}");
    println!("Token count:   {}", encoded.len());
    println!("Decoded text:  {decoded:?}");
    println!("Round trip:    {}", if matches { "OK" } else { "FAILED" });

    Ok(matches)
}
