use anyhow::{bail, Result};
use std::str::FromStr;

use alloy::primitives::U256;

pub fn join_with_final<T>(separator: &str, final_separator: &str, strings: Vec<T>) -> String
where
    T: std::string::ToString,
{
    if strings.is_empty() {
        return "".to_string();
    }
    if strings.len() == 1 {
        return strings[0].to_string();
    }
    let mut result = strings[0].to_string();
    for s in strings[1..strings.len() - 1].iter() {
        result.push_str(separator);
        result.push_str(&s.to_string());
    }
    result.push_str(final_separator);
    result.push_str(&strings[strings.len() - 1].to_string());
    result
}

pub fn parse_rational_literal(whole: &str, raw_fraction: &str, raw_exponent: &str) -> Result<U256> {
    let mut n = if whole.is_empty() {
        U256::from(0)
    } else {
        U256::from_str(whole)?
    };
    let exponent = if raw_exponent.is_empty() {
        U256::from(0)
    } else {
        U256::from_str(raw_exponent)?
    };
    n *= U256::from(10).pow(exponent);

    if !raw_fraction.is_empty() {
        let removed_zeros = raw_fraction.trim_end_matches('0');
        let decimals_count = U256::from(removed_zeros.len());
        let fraction = U256::from_str(removed_zeros)?;
        if decimals_count > exponent {
            bail!("fraction has more digits than decimals");
        }
        let adjusted_fraction = fraction * U256::from(10).pow(exponent - decimals_count);
        n += adjusted_fraction;
    };

    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_with_final() {
        assert_eq!(
            join_with_final(", ", " and ", vec!["a", "b", "c"]),
            "a, b and c"
        );
        assert_eq!(join_with_final(", ", " and ", vec!["a", "b"]), "a and b");
        assert_eq!(join_with_final(", ", " and ", vec!["a"]), "a");
    }

    #[test]
    fn test_parse_rational_literal() {
        // 1e3
        assert_eq!(
            parse_rational_literal("1", "", "3").unwrap(),
            U256::from(1000)
        );
        // 123
        assert_eq!(
            parse_rational_literal("123", "", "").unwrap(),
            U256::from(123)
        );
        // 1.2e3
        assert_eq!(
            parse_rational_literal("1", "2", "3").unwrap(),
            U256::from(1200)
        );
        // 1.0
        assert_eq!(
            parse_rational_literal("1", "0", "").unwrap(),
            U256::from(1)
        );
        // 1.01e3
        assert_eq!(
            parse_rational_literal("1", "01", "3").unwrap(),
            U256::from(1010)
        );
        // 1.1234e4
        assert_eq!(
            parse_rational_literal("1", "1234", "4").unwrap(),
            U256::from(11234)
        );
        // 1.12340e4
        assert_eq!(
            parse_rational_literal("1", "12340", "4").unwrap(),
            U256::from(11234)
        );
        // 1.1234e5
        assert_eq!(
            parse_rational_literal("1", "1234", "5").unwrap(),
            U256::from(112340)
        );
        // 1.01234e5
        assert_eq!(
            parse_rational_literal("1", "01234", "5").unwrap(),
            U256::from(101234)
        );
        // .1e3
        assert_eq!(
            parse_rational_literal("", "1", "3").unwrap(),
            U256::from(100)
        );
    }
}
