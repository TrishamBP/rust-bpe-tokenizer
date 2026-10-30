// Token vocabulary

/// The numeric ID of a token.
pub type TokenId = u32;

/// Maps each token ID to the bytes that the token represents.
/// The ID of a token is its index in `tokens`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    tokens: Vec<Vec<u8>>,
}

impl Vocabulary {
    /// Creates a vocabulary with one token for each byte value. Byte `b` has ID `b`.
    pub fn new() -> Self {
        let tokens = (0..=255u8).map(|byte| vec![byte]).collect();
        Vocabulary { tokens }
    }

    /// Returns the number of tokens in the vocabulary.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Returns the ID that the next added token will get.
    pub fn next_id(&self) -> TokenId {
        self.tokens.len() as TokenId
    }

    /// Returns the bytes of the token with ID `id`, or `None` if the ID is unknown.
    pub fn get(&self, id: TokenId) -> Option<&[u8]> {
        self.tokens.get(id as usize).map(|bytes| bytes.as_slice())
    }

    /// Returns the ID of the token whose bytes are `bytes`, or `None` if no token matches.
    /// If two tokens have the same bytes, returns the smaller ID.
    pub fn id_of(&self, bytes: &[u8]) -> Option<TokenId> {
        self.tokens
            .iter()
            .position(|token| token.as_slice() == bytes)
            .map(|index| index as TokenId)
    }

    /// Adds a token made of the bytes of `pair.0` followed by the bytes of `pair.1`.
    /// Returns the new ID, or `None` if either ID is unknown.
    pub fn add_merge(&mut self, pair: (TokenId, TokenId)) -> Option<TokenId> {
        let mut bytes = self.get(pair.0)?.to_vec();
        bytes.extend_from_slice(self.get(pair.1)?);
        let id = self.next_id();
        self.tokens.push(bytes);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_256_byte_tokens() {
        let vocab = Vocabulary::new();
        assert_eq!(vocab.next_id(), 256);
        assert_eq!(vocab.get(0), Some(&[0][..]));
        assert_eq!(vocab.get(97), Some(&b"a"[..]));
        assert_eq!(vocab.get(255), Some(&[255][..]));
        assert_eq!(vocab.get(256), None);
    }

    #[test]
    fn first_merge_gets_id_256() {
        let mut vocab = Vocabulary::new();
        assert_eq!(vocab.add_merge((97, 110)), Some(256));
        assert_eq!(vocab.get(256), Some(&b"an"[..]));
        assert_eq!(vocab.next_id(), 257);
    }

    #[test]
    fn merged_tokens_can_merge_again() {
        let mut vocab = Vocabulary::new();
        let an = vocab.add_merge((97, 110)).unwrap();
        let anan = vocab.add_merge((an, an)).unwrap();
        assert_eq!(anan, 257);
        assert_eq!(vocab.get(anan), Some(&b"anan"[..]));
    }

    #[test]
    fn finds_id_from_bytes() {
        let mut vocab = Vocabulary::new();
        let an = vocab.add_merge((97, 110)).unwrap();
        assert_eq!(vocab.id_of(b"a"), Some(97));
        assert_eq!(vocab.id_of(b"an"), Some(an));
        assert_eq!(vocab.id_of(b"xyz"), None);
        assert_eq!(vocab.id_of(b""), None);
    }

    #[test]
    fn merge_with_unknown_id_is_rejected() {
        let mut vocab = Vocabulary::new();
        assert_eq!(vocab.add_merge((97, 999)), None);
        assert_eq!(vocab.next_id(), 256);
    }
}
