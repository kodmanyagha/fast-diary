pub const AUTHOR_NAME: &str = "Emir Buğra Köksalan";
pub const AUTHOR_EMAIL: &str = "kodmanyagha@gmail.com";
pub const REPOSITORY_URL: &str = "https://github.com/kodmanyagha/fast-diary";
pub const ISSUES_URL: &str = "https://github.com/kodmanyagha/fast-diary/issues";
pub const ETHEREUM_ADDRESS: &str = "0xa3Ecccc8A958309519659f0d317A0B5eEEd43a26";
pub const SOLANA_ADDRESS: &str = "A6Q6aYPihLpvZhBvgXbFhxeXVTVb9T5t5jnciJEJ5RXB";

#[cfg(test)]
mod tests {
    use super::*;

    const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    #[test]
    fn the_issue_page_belongs_to_the_repository() {
        assert!(REPOSITORY_URL.starts_with("https://github.com/"));
        assert_eq!(ISSUES_URL, format!("{REPOSITORY_URL}/issues"));
    }

    #[test]
    fn the_email_address_has_a_name_and_a_domain() {
        assert!(AUTHOR_EMAIL
            .split_once('@')
            .is_some_and(|(name, domain)| !name.is_empty() && domain.contains('.')));
    }

    #[test]
    fn the_ethereum_address_is_0x_and_forty_hex_digits() {
        let digits = ETHEREUM_ADDRESS.strip_prefix("0x");

        assert!(digits.is_some_and(|digits| {
            digits.len() == 40 && digits.chars().all(|digit| digit.is_ascii_hexdigit())
        }));
    }

    #[test]
    fn the_solana_address_is_a_base58_key_of_32_to_44_characters() {
        assert!((32..=44).contains(&SOLANA_ADDRESS.len()));
        assert!(SOLANA_ADDRESS
            .chars()
            .all(|character| BASE58_ALPHABET.contains(character)));
    }
}
