// Binary save and load of a trained tokenizer
//
// File layout, version 1. All integers are little-endian.
//
//   magic               8 bytes   "RSBPETOK"
//   format version      u32       1
//   initial vocab size  u32       256
//   vocabulary count    u32       N
//   N entries:          id u32, byte length u32, bytes
//   merge count         u32       M (must equal N - 256)
//   M merge rules:      left u32, right u32, new id u32, frequency u64
//   target vocab size   u64
//   max merges          u8 flag (0 = none, 1 = set), then u64 if the flag is 1
//   corpus path         byte length u32, UTF-8 bytes

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::bpe::{INITIAL_VOCAB_SIZE, MergeRule, Tokenizer, TrainingConfig};
use super::vocabulary::{TokenId, Vocabulary};

/// The default location of the trained Shakespeare tokenizer.
pub const DEFAULT_MODEL_PATH: &str = "models/shakespeare_bpe.bin";

/// The first 8 bytes of every tokenizer file.
pub const MAGIC: [u8; 8] = *b"RSBPETOK";

/// The format version that this code writes and reads.
pub const FORMAT_VERSION: u32 = 1;

/// The smallest possible vocabulary entry: ID (4) + length (4) + one byte (1).
const MIN_ENTRY_SIZE: usize = 9;

/// The size of one merge rule: three u32 values (12) + one u64 value (8).
const MERGE_RULE_SIZE: usize = 20;

/// The reasons why saving or loading can fail.
#[derive(Debug)]
pub enum SerializationError {
    /// The file could not be read or written.
    Io(io::Error),
    /// The file does not start with `MAGIC`.
    InvalidMagic,
    /// The file has a format version that this code cannot read.
    UnsupportedVersion(u32),
    /// The file ends before all data is read.
    Truncated,
    /// The file has a value that is not permitted.
    InvalidData(String),
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationError::Io(error) => write!(f, "I/O error: {error}"),
            SerializationError::InvalidMagic => {
                write!(f, "not a tokenizer file (bad magic header)")
            }
            SerializationError::UnsupportedVersion(version) => {
                write!(f, "unsupported format version {version}")
            }
            SerializationError::Truncated => write!(f, "the file is truncated"),
            SerializationError::InvalidData(message) => write!(f, "invalid data: {message}"),
        }
    }
}

impl std::error::Error for SerializationError {}

impl From<io::Error> for SerializationError {
    fn from(error: io::Error) -> Self {
        SerializationError::Io(error)
    }
}

/// Writes `tokenizer` to the file at `path`. Creates the parent directory if necessary.
pub fn save(tokenizer: &Tokenizer, path: &Path) -> Result<(), SerializationError> {
    let bytes = to_bytes(tokenizer)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

/// Reads a tokenizer from the file at `path`.
pub fn load(path: &Path) -> Result<Tokenizer, SerializationError> {
    let bytes = fs::read(path)?;
    from_bytes(&bytes)
}

/// Converts `tokenizer` into the binary format.
pub fn to_bytes(tokenizer: &Tokenizer) -> Result<Vec<u8>, SerializationError> {
    let vocabulary = &tokenizer.vocabulary;
    let config = &tokenizer.config;
    let mut out = Vec::new();

    out.extend_from_slice(&MAGIC);
    write_u32(&mut out, FORMAT_VERSION);
    write_u32(&mut out, INITIAL_VOCAB_SIZE as u32);

    write_u32(&mut out, length_to_u32(vocabulary.len())?);
    for id in 0..vocabulary.next_id() {
        let bytes = vocabulary
            .get(id)
            .ok_or_else(|| invalid(format!("vocabulary has no token {id}")))?;
        write_u32(&mut out, id);
        write_u32(&mut out, length_to_u32(bytes.len())?);
        out.extend_from_slice(bytes);
    }

    write_u32(&mut out, length_to_u32(tokenizer.merges.len())?);
    for rule in &tokenizer.merges {
        write_u32(&mut out, rule.left);
        write_u32(&mut out, rule.right);
        write_u32(&mut out, rule.new_id);
        write_u64(&mut out, rule.frequency as u64);
    }

    write_u64(&mut out, config.target_vocab_size as u64);
    match config.max_merges {
        None => out.push(0),
        Some(max_merges) => {
            out.push(1);
            write_u64(&mut out, max_merges as u64);
        }
    }
    let path = config
        .corpus_path
        .to_str()
        .ok_or_else(|| invalid("corpus path is not valid UTF-8".to_string()))?;
    write_u32(&mut out, length_to_u32(path.len())?);
    out.extend_from_slice(path.as_bytes());

    Ok(out)
}

/// Reads a tokenizer from bytes in the binary format.
/// Validates every length, ID and merge reference. Does not retrain.
pub fn from_bytes(bytes: &[u8]) -> Result<Tokenizer, SerializationError> {
    let mut reader = Reader { rest: bytes };

    if reader.take(MAGIC.len())? != MAGIC {
        return Err(SerializationError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != FORMAT_VERSION {
        return Err(SerializationError::UnsupportedVersion(version));
    }
    let initial_size = reader.read_u32()? as usize;
    if initial_size != INITIAL_VOCAB_SIZE {
        return Err(invalid(format!(
            "initial vocabulary size is {initial_size}, expected {INITIAL_VOCAB_SIZE}"
        )));
    }

    let vocab_count = reader.read_u32()? as usize;
    if vocab_count < INITIAL_VOCAB_SIZE {
        return Err(invalid(format!(
            "vocabulary has {vocab_count} tokens, expected at least {INITIAL_VOCAB_SIZE}"
        )));
    }
    reader.check_count(vocab_count, MIN_ENTRY_SIZE)?;
    let mut entries = Vec::with_capacity(vocab_count);
    for index in 0..vocab_count {
        let id = reader.read_u32()?;
        if id as usize != index {
            return Err(invalid(format!("vocabulary entry {index} has ID {id}")));
        }
        let length = reader.read_u32()? as usize;
        if length == 0 {
            return Err(invalid(format!("token {id} has no bytes")));
        }
        entries.push(reader.take(length)?);
    }

    let merge_count = reader.read_u32()? as usize;
    if merge_count != vocab_count - INITIAL_VOCAB_SIZE {
        return Err(invalid(format!(
            "{merge_count} merge rules for {vocab_count} tokens"
        )));
    }
    reader.check_count(merge_count, MERGE_RULE_SIZE)?;
    let mut vocabulary = Vocabulary::new();
    let mut merges = Vec::with_capacity(merge_count);
    for index in 0..merge_count {
        let left = reader.read_u32()?;
        let right = reader.read_u32()?;
        let new_id = reader.read_u32()?;
        let frequency = u64_to_usize(reader.read_u64()?)?;
        if new_id != vocabulary.next_id() {
            return Err(invalid(format!(
                "merge {index} creates ID {new_id}, expected {}",
                vocabulary.next_id()
            )));
        }
        if vocabulary.add_merge((left, right)).is_none() {
            return Err(invalid(format!(
                "merge {index} refers to an unknown token: ({left}, {right})"
            )));
        }
        merges.push(MergeRule {
            left,
            right,
            new_id,
            frequency,
        });
    }

    for (id, &saved_bytes) in entries.iter().enumerate() {
        if vocabulary.get(id as TokenId) != Some(saved_bytes) {
            return Err(invalid(format!(
                "token {id} bytes do not match its merge rule"
            )));
        }
    }

    let target_vocab_size = u64_to_usize(reader.read_u64()?)?;
    let max_merges = match reader.read_u8()? {
        0 => None,
        1 => Some(u64_to_usize(reader.read_u64()?)?),
        flag => return Err(invalid(format!("max merges flag is {flag}"))),
    };
    let path_length = reader.read_u32()? as usize;
    let path = std::str::from_utf8(reader.take(path_length)?)
        .map_err(|_| invalid("corpus path is not valid UTF-8".to_string()))?;

    if !reader.rest.is_empty() {
        return Err(invalid(format!(
            "{} unexpected bytes after the end of the data",
            reader.rest.len()
        )));
    }

    Ok(Tokenizer {
        config: TrainingConfig {
            corpus_path: PathBuf::from(path),
            target_vocab_size,
            max_merges,
        },
        vocabulary,
        merges,
    })
}

/// Reads values from the front of a byte slice. Every read checks the remaining length.
struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Removes `count` bytes from the front and returns them.
    fn take(&mut self, count: usize) -> Result<&'a [u8], SerializationError> {
        if count > self.rest.len() {
            return Err(SerializationError::Truncated);
        }
        let (taken, rest) = self.rest.split_at(count);
        self.rest = rest;
        Ok(taken)
    }

    /// Returns an error if `count` items of at least `item_size` bytes cannot fit in the rest.
    /// Call this before allocating memory for `count` items.
    fn check_count(&self, count: usize, item_size: usize) -> Result<(), SerializationError> {
        if count > self.rest.len() / item_size {
            return Err(SerializationError::Truncated);
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, SerializationError> {
        Ok(self.take(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, SerializationError> {
        let mut buffer = [0; 4];
        buffer.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(buffer))
    }

    fn read_u64(&mut self) -> Result<u64, SerializationError> {
        let mut buffer = [0; 8];
        buffer.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(buffer))
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn length_to_u32(length: usize) -> Result<u32, SerializationError> {
    u32::try_from(length).map_err(|_| invalid(format!("length {length} does not fit in u32")))
}

fn u64_to_usize(value: u64) -> Result<usize, SerializationError> {
    usize::try_from(value).map_err(|_| invalid(format!("value {value} does not fit in usize")))
}

fn invalid(message: String) -> SerializationError {
    SerializationError::InvalidData(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::bpe::train;

    /// Byte offset of the format version.
    const VERSION_OFFSET: usize = 8;
    /// Byte offset of the vocabulary count.
    const VOCAB_COUNT_OFFSET: usize = 16;

    fn trained_tokenizer() -> Tokenizer {
        let config = TrainingConfig {
            target_vocab_size: 270,
            max_merges: Some(50),
            ..TrainingConfig::default()
        };
        let result = train("banana bandana cabana banana bandana", &config);
        Tokenizer {
            config,
            vocabulary: result.vocabulary,
            merges: result.merges,
        }
    }

    /// Returns a path in the system temporary directory. The real model is never used.
    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rust_bpe_test_{}_{name}.bin", std::process::id()))
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn save_writes_file_with_magic_header() {
        let path = temp_path("save");
        save(&trained_tokenizer(), &path).expect("save should succeed");
        let bytes = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(&bytes[..8], &MAGIC);
    }

    #[test]
    fn load_restores_the_same_tokenizer() {
        let original = trained_tokenizer();
        let path = temp_path("round_trip");
        save(&original, &path).unwrap();
        let loaded = load(&path).expect("load should succeed");
        fs::remove_file(&path).unwrap();

        assert_eq!(loaded.vocabulary, original.vocabulary);
        assert_eq!(loaded.merges, original.merges);
        assert_eq!(loaded.config, original.config);
        assert_eq!(loaded, original);
    }

    #[test]
    fn loaded_tokenizer_encodes_and_decodes_the_same() {
        let original = trained_tokenizer();
        let loaded = from_bytes(&to_bytes(&original).unwrap()).unwrap();
        for text in [
            "banana bandana",
            "cabana",
            "an unseen sentence",
            "café 🦀",
            "",
        ] {
            let encoded = original.encode(text);
            assert_eq!(loaded.encode(text), encoded);
            assert_eq!(loaded.decode(&encoded), original.decode(&encoded));
            assert_eq!(loaded.decode(&encoded).unwrap(), text);
        }
    }

    #[test]
    fn tokenizer_without_merges_round_trips() {
        let original = Tokenizer {
            config: TrainingConfig::default(),
            vocabulary: Vocabulary::new(),
            merges: Vec::new(),
        };
        let loaded = from_bytes(&to_bytes(&original).unwrap()).unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn empty_vocabulary_is_rejected() {
        // A vocabulary always has the 256 byte tokens. An empty one is not supported.
        let mut bytes = to_bytes(&trained_tokenizer()).unwrap();
        put_u32(&mut bytes, VOCAB_COUNT_OFFSET, 0);
        assert!(matches!(
            from_bytes(&bytes),
            Err(SerializationError::InvalidData(_))
        ));
    }

    #[test]
    fn invalid_magic_is_rejected() {
        let mut bytes = to_bytes(&trained_tokenizer()).unwrap();
        bytes[0] = b'X';
        assert!(matches!(
            from_bytes(&bytes),
            Err(SerializationError::InvalidMagic)
        ));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let mut bytes = to_bytes(&trained_tokenizer()).unwrap();
        put_u32(&mut bytes, VERSION_OFFSET, 2);
        assert!(matches!(
            from_bytes(&bytes),
            Err(SerializationError::UnsupportedVersion(2))
        ));
    }

    #[test]
    fn every_truncated_prefix_is_rejected() {
        let bytes = to_bytes(&trained_tokenizer()).unwrap();
        for length in 0..bytes.len() {
            assert!(
                from_bytes(&bytes[..length]).is_err(),
                "a file cut at {length} bytes must be rejected"
            );
        }
    }

    #[test]
    fn huge_count_is_rejected_before_allocation() {
        let mut bytes = to_bytes(&trained_tokenizer()).unwrap();
        put_u32(&mut bytes, VOCAB_COUNT_OFFSET, u32::MAX);
        assert!(matches!(
            from_bytes(&bytes),
            Err(SerializationError::Truncated)
        ));
    }

    #[test]
    fn invalid_token_reference_is_rejected() {
        let mut vocabulary = Vocabulary::new();
        vocabulary.add_merge((97, 98)).unwrap();
        let broken = Tokenizer {
            config: TrainingConfig::default(),
            vocabulary,
            merges: vec![MergeRule {
                left: 97,
                right: 999,
                new_id: 256,
                frequency: 1,
            }],
        };
        let result = from_bytes(&to_bytes(&broken).unwrap());
        assert!(matches!(result, Err(SerializationError::InvalidData(_))));
    }

    #[test]
    fn mismatched_token_bytes_are_rejected() {
        let mut vocabulary = Vocabulary::new();
        vocabulary.add_merge((97, 98)).unwrap(); // token 256 = "ab"
        let broken = Tokenizer {
            config: TrainingConfig::default(),
            vocabulary,
            merges: vec![MergeRule {
                left: 98, // the rule says "ba"
                right: 97,
                new_id: 256,
                frequency: 1,
            }],
        };
        let result = from_bytes(&to_bytes(&broken).unwrap());
        assert!(matches!(result, Err(SerializationError::InvalidData(_))));
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let mut bytes = to_bytes(&trained_tokenizer()).unwrap();
        bytes.push(0);
        assert!(matches!(
            from_bytes(&bytes),
            Err(SerializationError::InvalidData(_))
        ));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let result = load(&temp_path("does_not_exist"));
        match result {
            Err(SerializationError::Io(error)) => assert_eq!(error.kind(), io::ErrorKind::NotFound),
            other => panic!("expected a NotFound I/O error, got {other:?}"),
        }
    }
}
