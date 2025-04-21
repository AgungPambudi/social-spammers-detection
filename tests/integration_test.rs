#[cfg(test)]
mod tests {
    // Import the function we want to test from the parent module
    use super::*;
    // We also need the trait import within the test module scope
    use unicode_normalization::UnicodeNormalization;

    #[test]
    fn test_contains_unicode_abuse_positive() {
        // Fullwidth Latin characters - visually similar but different Unicode points
        let abusive_text = "Ｈｅｌｌｏ Ｗｏｒｌｄ"; // Uses fullwidth characters
        assert!(contains_unicode_abuse(abusive_text), "Should detect fullwidth characters as abuse");

        // Mixed normal and fullwidth
        let mixed_text = "Ｌｏｏｋ @ this!";
        assert!(contains_unicode_abuse(mixed_text), "Should detect mixed fullwidth characters");

        // Other visually deceptive characters (if applicable, depends on normalization form)
        // Example: Using Cyrillic 'а' instead of Latin 'a' might sometimes be caught
        // depending on the specific characters and normalization form, NFKD often handles these.
        let deceptive_text = "pаypаl"; // Cyrillic 'а' used
         // Check if NFKD normalization specifically changes this string
        assert_eq!(deceptive_text.nfkd().collect::<String>(), "paypal");
        assert!(contains_unicode_abuse(deceptive_text), "Should detect visually similar Cyrillic characters");

    }

    #[test]
    fn test_contains_unicode_abuse_negative() {
        // Standard ASCII text
        let normal_text_ascii = "Hello World 123 !@#";
        assert!(!contains_unicode_abuse(normal_text_ascii), "Standard ASCII should not be flagged");

        // Standard Unicode text (Chinese) - no normalization changes expected
        let normal_text_unicode = "你好世界";
        assert!(!contains_unicode_abuse(normal_text_unicode), "Standard Unicode should not be flagged");

        // Empty string
        let empty_text = "";
        assert!(!contains_unicode_abuse(empty_text), "Empty string should not be flagged");

        // String with only standard symbols
        let symbols_text = "!@#$%^&*()_+";
         assert!(!contains_unicode_abuse(symbols_text), "String with only symbols should not be flagged");
    }
}