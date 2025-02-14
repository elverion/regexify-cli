use std::collections::HashMap;
use clap::Parser;
use rand::Rng;
use regex::Regex;
use rand::prelude::IteratorRandom;

/// Output a pseudo-random string conforming to the regex-like input pattern
#[derive(Parser, Debug)]
#[command(name = "regexify-cli")]
#[command(about = "Write some about line later.")]
struct Cli {
    pattern: String,
}

fn main() {
    let args = Cli::parse();
    println!("{}", process_pattern(&args.pattern));
}

fn process_pattern(pattern: &str) -> String {
    let re = Regex::new(r"\[([^\]]+)\](\{(\d+)(?:,(\d+))?\})?").unwrap();
    let alt_re = Regex::new(r"\(([^)]+)\)").unwrap();
    let mut rng = rand::rng();
    let mut result = String::new();
    let mut last_match_end = 0;

    // Define common token mappings
    let token_map: HashMap<&str, &str> = [
        ("\\d", "0123456789"),   // Digits
        ("\\s", " \t\n\r"), // Whitespace characters
        ("\\w", "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"), // Word characters
    ]
        .iter()
        .cloned()
        .collect();

    // Process alternatives first
    let mut pattern_with_alts = String::new();
    let mut last_end = 0;

    for alt_caps in alt_re.captures_iter(pattern) {
        let full_match = alt_caps.get(0).unwrap();
        let options = &alt_caps[1];

        // Split the alternatives by the pipe symbol '|'
        let options_list: Vec<&str> = options.split('|').collect();

        // Randomly pick one of the alternatives
        let random_option = options_list.into_iter().choose(&mut rng).unwrap();

        // Append the part before the alternative
        pattern_with_alts.push_str(&pattern[last_end..full_match.start()]);
        pattern_with_alts.push_str(random_option);

        last_end = full_match.end();
    }

    // Append the rest of the pattern after all alternatives
    pattern_with_alts.push_str(&pattern[last_end..]);

    // Now replace common tokens with their corresponding character sets
    let mut expanded_pattern = pattern_with_alts.clone();
    for (token, replacement) in token_map.iter() {
        expanded_pattern = expanded_pattern.replace(token, replacement);
    }

    // Now process the remaining pattern (with common tokens replaced)
    for caps in re.captures_iter(&expanded_pattern) {
        let full_match = caps.get(0).unwrap();
        let char_class = &caps[1];

        // Extract min and max lengths, handling cases where no quantifier is provided
        let (min_length, max_length) = if let Some(min_caps) = caps.get(3) {
            let min: usize = min_caps.as_str().parse().unwrap_or(1);
            let max: usize = if let Some(max_caps) = caps.get(4) {
                max_caps.as_str().parse().unwrap_or(min)
            } else {
                min
            };
            (min, max)
        } else {
            // No quantifier, so we assume {1} (1 character)
            (1, 1)
        };

        // If min_length == max_length, we should use that value directly, not randomize
        let length = if min_length == max_length {
            min_length // Use the exact value if min and max are the same
        } else {
            rng.random_range(min_length..=max_length) // Random length within the range
        };

        // Append literal text before the match
        result.push_str(&expanded_pattern[last_match_end..full_match.start()]);

        // Generate character pool from character class
        let mut char_pool = String::new();
        let mut i = 0;
        while i < char_class.len() {
            if i + 2 < char_class.len() && char_class.as_bytes()[i + 1] == b'-' {
                // Handle ranges like a-j, k-z, etc.
                let start = char_class.chars().nth(i).unwrap();
                let end = char_class.chars().nth(i + 2).unwrap();
                char_pool.push_str(&(start..=end).collect::<String>());
                i += 3; // Skip past the range (e.g., "a-j")
            } else {
                // Handle individual characters (no range, just single characters)
                char_pool.push(char_class.chars().nth(i).unwrap());
                i += 1;
            }
        }

        // Generate random string based on the pool
        let random_segment: String = (0..length)
            .map(|_| char_pool.chars().choose(&mut rng).unwrap())
            .collect();

        result.push_str(&random_segment);
        last_match_end = full_match.end();
    }

    result.push_str(&expanded_pattern[last_match_end..]);
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_pattern_handles_common_tokens() {
        let out = process_pattern("[\\d]{10}");
        assert!(out.chars().all(|c| matches!(c, '0'..='9')));

        let out = process_pattern("[\\s]{10}");
        assert!(out.chars().all(|c| c.is_whitespace()));

        let out = process_pattern("[\\w]{10}");
        assert!(out.chars().all(|c| matches!(c, 'a'..='z' | 'A' ..='Z' | '0'..='9' | '_')));
    }

    #[test]
    fn test_process_pattern_handles_sets() {
        let out = process_pattern("[abcdef]");
        assert!(out.chars().all(|c| matches!(c, 'a'..='f')));

        let out = process_pattern("[abcdef]{100}");
        assert!(out.chars().all(|c| matches!(c, 'a'..='f')));

        let out = process_pattern("[abcdef0-9]{100}");
        assert!(out.chars().all(|c| matches!(c, 'a'..='f' | '0' ..='9')));

        let out = process_pattern("[a-ef-jk-pqz0-45-9]{100}");
        assert!(out.chars().all(|c| matches!(c, 'a'..='z' | '0' ..='9')));
    }

    #[test]
    fn test_process_pattern_handles_quantifier() {
        let out = process_pattern("[a-z]{4}");
        assert_eq!(out.len(), 4);
        assert!(out.chars().all(|c| matches!(c, 'a'..='z')));

        let out = process_pattern("[a-z]{4,8}");
        assert!(out.len() > 3 && out.len() < 9);
        assert!(out.chars().all(|c| matches!(c, 'a'..='z')));
    }

    #[test]
    fn test_process_pattern_handles_quantifier_range() {
        let out = process_pattern("[a-z]{10,16}");
        assert!(out.len() > 9 && out.len() < 17);
        assert!(out.chars().all(|c| matches!(c, 'a'..='z')));
    }

    #[test]
    fn test_process_pattern_handles_multiple_captures() {
        let out = process_pattern("[a-z]{2}[0-9]{3}");
        assert_eq!(out.len(), 5);
        assert!(out.chars().take(2).all(|c| c.is_ascii_lowercase()), "First two chars should be a-z");
        assert!(out.chars().skip(2).all(|c| c.is_ascii_digit()), "Last three chars should be 0-9");
    }

    #[test]
    fn test_process_pattern_handles_alternative_options() {
        let out = process_pattern("(this|that|red|blue)");
        assert!(out.eq("this") || out.eq("that") || out.eq("red") || out.eq("blue"));
    }
}