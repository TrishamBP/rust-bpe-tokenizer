//! Benchmarks for training, encoding, decoding, saving and loading.
//!
//! Run with: cargo bench
//!
//! This harness uses only the standard library. Each benchmark first runs
//! warm-up iterations that are not measured. Then it runs measured iterations
//! and reports the minimum, median, mean and maximum time.

use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

use rust_tokenizer::tokenizer::bpe::{Tokenizer, TrainingConfig, train};
use rust_tokenizer::tokenizer::serialization::{from_bytes, load, save, to_bytes};

const CORPUS_PATH: &str = "data/shakespeare.txt";
const SHORT_TEXT: &str = "To be, or not to be, that is the question.";
const MEDIUM_BYTES: usize = 10_000;
const LARGE_BYTES: usize = 100_000;

/// Timing results for one benchmark.
struct Stats {
    iterations: usize,
    min: Duration,
    median: Duration,
    mean: Duration,
    max: Duration,
}

/// Runs `operation` `warmup` times without timing, then `iterations` times with timing.
/// Returns the statistics and the output of the last run.
fn measure<T>(warmup: usize, iterations: usize, mut operation: impl FnMut() -> T) -> (Stats, T) {
    for _ in 0..warmup {
        black_box(operation());
    }
    let mut times = Vec::with_capacity(iterations);
    let mut output = None;
    for _ in 0..iterations {
        let start = Instant::now();
        let value = black_box(operation());
        times.push(start.elapsed());
        output = Some(value);
    }
    times.sort();
    let total: Duration = times.iter().sum();
    let stats = Stats {
        iterations,
        min: times[0],
        median: times[iterations / 2],
        mean: total / iterations as u32,
        max: times[iterations - 1],
    };
    (stats, output.expect("iterations must be at least 1"))
}

fn print_stats(name: &str, stats: &Stats) {
    println!(
        "{name:<28} iterations {:>5} | min {:>12.3?} | median {:>12.3?} | mean {:>12.3?} | max {:>12.3?}",
        stats.iterations, stats.min, stats.median, stats.mean, stats.max
    );
}

/// Returns the longest prefix of `text` that has at most `max_bytes` bytes and ends on a character boundary.
fn prefix(text: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn per_second(amount: usize, time: Duration) -> f64 {
    amount as f64 / time.as_secs_f64()
}

fn main() {
    println!("== Environment ==");
    println!("architecture:     {}", std::env::consts::ARCH);
    println!("operating system: {}", std::env::consts::OS);
    println!(
        "logical CPUs:     {}",
        std::thread::available_parallelism().map_or(0, |n| n.get())
    );
    println!(
        "build profile:    {}",
        if cfg!(debug_assertions) {
            "debug (results are not representative)"
        } else {
            "release (optimized)"
        }
    );
    println!();

    // The corpus is read once, before any timing starts.
    let corpus = fs::read_to_string(CORPUS_PATH).expect("run from the project root");
    let config = TrainingConfig::default();

    println!("== A. Training ==");
    let (stats, result) = measure(1, 5, || train(&corpus, &config));
    println!();
    print_stats("train (full corpus)", &stats);
    println!("corpus size:      {} bytes", corpus.len());
    println!("vocabulary size:  {}", result.vocabulary.len());
    println!("merges:           {}", result.merges.len());
    println!(
        "time per merge:   {:.3?} (median / merges)",
        stats.median / result.merges.len() as u32
    );
    println!();

    let tokenizer = Tokenizer {
        config: config.clone(),
        vocabulary: result.vocabulary,
        merges: result.merges,
    };

    println!("== B. Encoding and C. Decoding ==");
    let samples = [
        ("short sentence", SHORT_TEXT, 200, 5_000),
        ("medium (10 KB)", prefix(&corpus, MEDIUM_BYTES), 5, 100),
        ("large (100 KB)", prefix(&corpus, LARGE_BYTES), 2, 20),
        ("full corpus", corpus.as_str(), 1, 5),
    ];
    for (name, text, warmup, iterations) in samples {
        let (encode_stats, encoded) = measure(warmup, iterations, || tokenizer.encode(text));
        let (decode_stats, decoded) =
            measure(warmup, iterations, || tokenizer.decode(&encoded).unwrap());
        assert_eq!(decoded, text, "decoding must restore the input");

        print_stats(&format!("encode {name}"), &encode_stats);
        println!(
            "    {} bytes -> {} tokens | {:.0} bytes/s | {:.0} tokens/s (median)",
            text.len(),
            encoded.len(),
            per_second(text.len(), encode_stats.median),
            per_second(encoded.len(), encode_stats.median)
        );
        print_stats(&format!("decode {name}"), &decode_stats);
        println!(
            "    {:.0} bytes/s | {:.0} tokens/s (median) | output verified",
            per_second(text.len(), decode_stats.median),
            per_second(encoded.len(), decode_stats.median)
        );
    }
    println!();

    println!("== D. Serialization and E. Deserialization ==");
    let path = std::env::temp_dir().join(format!("rust_bpe_bench_{}.bin", std::process::id()));
    let (stats, ()) = measure(5, 100, || save(&tokenizer, &path).unwrap());
    print_stats("save (to file)", &stats);
    let file_size = fs::metadata(&path).unwrap().len();
    println!("file size:        {file_size} bytes");

    let (stats, loaded) = measure(5, 100, || load(&path).unwrap());
    print_stats("load (from file)", &stats);
    assert_eq!(
        loaded, tokenizer,
        "the loaded tokenizer must equal the original"
    );
    assert_eq!(loaded.encode(SHORT_TEXT), tokenizer.encode(SHORT_TEXT));
    println!("loaded tokenizer: verified equal to the original");
    fs::remove_file(&path).unwrap();

    let bytes = to_bytes(&tokenizer).unwrap();
    let (stats, _) = measure(5, 100, || to_bytes(&tokenizer).unwrap());
    print_stats("to_bytes (memory only)", &stats);
    let (stats, _) = measure(5, 100, || from_bytes(&bytes).unwrap());
    print_stats("from_bytes (memory only)", &stats);
    println!();

    println!("== F. Compression (full corpus) ==");
    let encoded = tokenizer.encode(&corpus);
    let original_bytes = corpus.len();
    let token_count = encoded.len();
    println!("original bytes:   {original_bytes}");
    println!("encoded tokens:   {token_count}");
    println!(
        "bytes per token:  {:.4}",
        original_bytes as f64 / token_count as f64
    );
    println!(
        "compression ratio (bytes / tokens): {:.4}",
        original_bytes as f64 / token_count as f64
    );
    println!(
        "token reduction:  {:.2}%",
        100.0 * (1.0 - token_count as f64 / original_bytes as f64)
    );
}
