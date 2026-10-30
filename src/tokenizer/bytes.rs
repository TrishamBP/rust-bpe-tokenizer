// UTF-8 byte-level representation

use std::str::Utf8Error;

use super::vocabulary::TokenId;

/// Returns the UTF-8 bytes of `text` without copying.
pub fn text_to_bytes(text: &str) -> &[u8] {
    text.as_bytes()
}

/// Returns one token ID for each UTF-8 byte of `text`. Byte `b` becomes ID `b`.
pub fn text_to_token_ids(text: &str) -> Vec<TokenId> {
    text_to_bytes(text)
        .iter()
        .map(|&byte| TokenId::from(byte))
        .collect()
}

/// Converts UTF-8 bytes back into a `String`, or returns an error if they are not valid UTF-8.
pub fn bytes_to_text(bytes: &[u8]) -> Result<String, Utf8Error> {
    let text = std::str::from_utf8(bytes)?;
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_to_bytes() {
        assert_eq!(text_to_bytes("hello"), [104, 101, 108, 108, 111]);
    }

    #[test]
    fn cat_to_bytes() {
        assert_eq!(text_to_bytes("cat"), [99, 97, 116]);
    }

    #[test]
    fn hello_world_to_bytes() {
        assert_eq!(
            text_to_bytes("hello world"),
            [104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]
        );
    }

    #[test]
    fn multibyte_character_uses_more_bytes_than_chars() {
        let text = "café";
        let bytes = text_to_bytes(text);
        assert_eq!(text.chars().count(), 4);
        assert_eq!(bytes.len(), 5);
        assert_eq!(bytes, [99, 97, 102, 0xC3, 0xA9]);
    }

    #[test]
    fn round_trip_preserves_text() {
        for original in ["hello", "cat", "hello world", "café"] {
            let bytes = text_to_bytes(original);
            let decoded = bytes_to_text(bytes).expect("valid UTF-8 should decode");
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn invalid_utf8_returns_error() {
        // 0xC3 starts a 2-byte sequence, but 0x28 ('(') is not a valid continuation byte.
        let invalid = [0xC3, 0x28];
        assert!(bytes_to_text(&invalid).is_err());

        // 0xFF never appears anywhere in valid UTF-8.
        assert!(bytes_to_text(&[0xFF]).is_err());
    }

    #[test]
    fn token_ids_match_bytes() {
        assert_eq!(text_to_token_ids("café"), vec![99, 97, 102, 0xC3, 0xA9]);
        assert!(text_to_token_ids("").is_empty());
    }
}
