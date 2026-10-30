// Initial text splitting

/// Splits text into words on whitespace, keeping punctuation attached.
pub fn pre_tokenize(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sentence_on_whitespace() {
        let text = "First Citizen:\nBefore we proceed,  hear me speak.";
        let words = pre_tokenize(text);
        assert_eq!(
            words,
            vec![
                "First", "Citizen:", "Before", "we", "proceed,", "hear", "me", "speak."
            ]
        );
    }
}
