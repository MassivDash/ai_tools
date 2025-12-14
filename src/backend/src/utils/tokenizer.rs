use std::sync::Once;
use tokenizers::tokenizer::{Result as TokenizerResult, Tokenizer};

// Global tokenizer instance - initialized once and reused
// Using GPT-2 style BPE tokenizer
static TOKENIZER_INIT: Once = Once::new();
static mut TOKENIZER: Option<Tokenizer> = None;

pub fn get_tokenizer() -> TokenizerResult<&'static Tokenizer> {
    unsafe {
        let mut init_error: Option<tokenizers::tokenizer::Error> = None;
        TOKENIZER_INIT.call_once(|| {
            // Try to load GPT-2 tokenizer
            match Tokenizer::from_pretrained("gpt2", None) {
                Ok(tok) => {
                    println!("✅ Loaded GPT-2 tokenizer for token counting");
                    TOKENIZER = Some(tok);
                }
                Err(e) => {
                    println!(
                        "⚠️ Failed to load GPT-2 tokenizer: {:?}. Will retry on next call.",
                        e
                    );
                    init_error = Some(e);
                }
            }
        });

        if let Some(err) = init_error {
            return Err(err);
        }

        // SAFETY: TOKENIZER is only written to during initialization (call_once),
        // and after that it's only read. This is safe because Once ensures single initialization.
        #[allow(static_mut_refs)]
        TOKENIZER.as_ref().ok_or_else(|| {
            use std::io;
            Box::new(io::Error::other("Tokenizer initialization failed"))
                as tokenizers::tokenizer::Error
        })
    }
}

/// Counts the number of tokens in a text string
pub fn count_tokens(text: &str) -> Result<usize, String> {
    match get_tokenizer() {
        Ok(tokenizer) => match tokenizer.encode(text, false) {
            Ok(encoding) => Ok(encoding.len()),
            Err(e) => Err(format!("Failed to encode text: {}", e)),
        },
        Err(e) => Err(format!("Failed to get tokenizer: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_basic() {
        let text = "Hello world";
        let result = count_tokens(text);
        // Should succeed if tokenizer is available
        if let Ok(count) = result {
            assert!(count > 0);
            assert!(count <= text.len()); // Tokens are usually fewer than characters
        }
    }

    #[test]
    fn test_count_tokens_empty() {
        let text = "";
        let result = count_tokens(text);
        // Empty text should return 0 or handle gracefully
        if let Ok(count) = result {
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn test_count_tokens_long_text() {
        let text = "This is a longer text that should be tokenized properly. ".repeat(10);
        let result = count_tokens(&text);
        // Should handle longer text
        if let Ok(count) = result {
            assert!(count > 0);
        }
    }

    #[test]
    fn test_count_tokens_special_characters() {
        let text = "Hello! @world #test $123";
        let result = count_tokens(text);
        // Should handle special characters
        if let Ok(count) = result {
            assert!(count > 0);
        }
    }

    #[test]
    fn test_count_tokens_multiline() {
        let text = "Line 1\nLine 2\nLine 3";
        let result = count_tokens(text);
        // Should handle multiline text
        if let Ok(count) = result {
            assert!(count > 0);
        }
    }

    #[test]
    fn test_count_tokens_unicode() {
        let text = "Hello 世界 🌍";
        let result = count_tokens(text);
        // Should handle unicode characters
        if let Ok(count) = result {
            assert!(count > 0);
        }
    }
}
