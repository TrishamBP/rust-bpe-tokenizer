// Token IDs to text

use std::fmt;
use std::str::Utf8Error;

use super::vocabulary::{TokenId, Vocabulary};

/// The reasons why decoding can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The vocabulary has no token with this ID.
    UnknownTokenId(TokenId),
    /// The bytes of the tokens are not valid UTF-8.
    InvalidUtf8(Utf8Error),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnknownTokenId(id) => write!(f, "unknown token ID {id}"),
            DecodeError::InvalidUtf8(error) => write!(f, "invalid UTF-8: {error}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Converts token IDs back into text.
/// Returns an error if an ID is unknown or if the bytes are not valid UTF-8.
pub fn decode(tokens: &[TokenId], vocabulary: &Vocabulary) -> Result<String, DecodeError> {
    let mut bytes = Vec::new();
    for &id in tokens {
        let token_bytes = vocabulary.get(id).ok_or(DecodeError::UnknownTokenId(id))?;
        bytes.extend_from_slice(token_bytes);
    }
    String::from_utf8(bytes).map_err(|error| DecodeError::InvalidUtf8(error.utf8_error()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vocabulary with "he" = 256, "ll" = 257, "hell" = 258.
    fn hello_vocabulary() -> Vocabulary {
        let mut vocab = Vocabulary::new();
        vocab.add_merge((104, 101)).unwrap();
        vocab.add_merge((108, 108)).unwrap();
        vocab.add_merge((256, 257)).unwrap();
        vocab
    }

    #[test]
    fn decodes_encoded_word() {
        assert_eq!(
            decode(&[258, 111], &hello_vocabulary()),
            Ok("hello".to_string())
        );
    }

    #[test]
    fn decodes_multiple_tokens_in_order() {
        // "he" + "ll" + "o" + " " + "hell" + "o"
        let tokens = [256, 257, 111, 32, 258, 111];
        assert_eq!(
            decode(&tokens, &hello_vocabulary()),
            Ok("hello hello".to_string())
        );
    }

    #[test]
    fn decodes_empty_sequence() {
        assert_eq!(decode(&[], &hello_vocabulary()), Ok(String::new()));
    }

    #[test]
    fn unknown_token_id_is_an_error() {
        assert_eq!(
            decode(&[104, 999], &hello_vocabulary()),
            Err(DecodeError::UnknownTokenId(999))
        );
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        // 0xC3 starts a 2-byte character, but 0x28 ('(') cannot continue it.
        let result = decode(&[0xC3, 0x28], &Vocabulary::new());
        assert!(matches!(result, Err(DecodeError::InvalidUtf8(_))));

        // The first byte of "é" alone is not a complete character.
        let result = decode(&[0xC3], &Vocabulary::new());
        assert!(matches!(result, Err(DecodeError::InvalidUtf8(_))));
    }
}
