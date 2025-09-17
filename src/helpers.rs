use tokio::io::{ self, AsyncBufReadExt, AsyncWriteExt, BufWriter, Stdout };

pub async fn handle_quotes(input: &str, stdout: &mut BufWriter<Stdout>) -> io::Result<String> {
    let mut final_input = input.to_string();

    loop {
        let quote_status = check_quotes(&final_input);

        match quote_status {
            QuoteStatus::Balanced => {
                break;
            }
            QuoteStatus::UnclosedSingle | QuoteStatus::UnclosedDouble => {
                // flag=true;
                stdout.write_all(b"> ").await?;
                stdout.flush().await?;

                let stdin = io::stdin();
                let mut reader = io::BufReader::new(stdin);
                let mut additional_input = String::new();
                reader.read_line(&mut additional_input).await?;

                // Remove the newline character
                if additional_input.ends_with('\n') {
                    additional_input.pop();
                    if additional_input.ends_with('\r') {
                        additional_input.pop();
                    }
                }

                final_input.push('\n');
                final_input.push_str(&additional_input);
            }
        }
    }

    Ok(process_shell_quotes(&final_input))
}

#[derive(Debug, PartialEq)]
enum QuoteStatus {
    Balanced,
    UnclosedSingle,
    UnclosedDouble,
}

fn check_quotes(input: &str) -> QuoteStatus {
    let mut single_quote_count = 0;
    let mut double_quote_count = 0;
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        match chars[i] {
            '\'' => {
                if !is_escaped(&chars, i) && double_quote_count == 0 {
                    single_quote_count += 1;
                }
            }
            '"' => {
                if !is_escaped(&chars, i) && single_quote_count ==0 {
                    double_quote_count += 1;
                }
            }
            _ => {}
        }
    }

    if single_quote_count % 2 != 0 {
        return QuoteStatus::UnclosedSingle;
    }

    if double_quote_count % 2 != 0 {
        return QuoteStatus::UnclosedDouble;
    }

    QuoteStatus::Balanced
}

fn is_escaped(chars: &[char], position: usize) -> bool {
    if position == 0 {
        return false;
    }

    let mut escape_count = 0;
    let mut pos = position;

    while pos > 0 && chars[pos - 1] == '\\' {
        escape_count += 1;
        pos -= 1;
    }

    escape_count % 2 != 0
}

fn process_shell_quotes(input: &str) -> String {
    if input.is_empty() {
        return input.to_string();
    }
    
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    
    while i < chars.len() {
        let ch = chars[i];
        
        match ch {
            '"' => {
                // Find the closing double quote
                if let Some(closing_pos) = find_closing_quote(&chars, i, '"') {
                    // Add content between quotes (without the quotes)
                    for j in (i + 1)..closing_pos {
                        result.push(chars[j]);
                    }
                    i = closing_pos + 1; // Skip past closing quote
                } else {
                    // No closing quote found, treat as literal
                    result.push(ch);
                    i += 1;
                }
            },
            '\'' => {
                // Find the closing single quote
                if let Some(closing_pos) = find_closing_quote(&chars, i, '\'') {
                    // Add content between quotes (without the quotes)
                    for j in (i + 1)..closing_pos {
                        result.push(chars[j]);
                    }
                    i = closing_pos + 1; // Skip past closing quote
                } else {
                    // No closing quote found, treat as literal
                    result.push(ch);
                    i += 1;
                }
            },
            '\\' => {
                // Handle escaped characters
                if i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    match next_char {
                        '"' | '\'' | '\\' => {
                            // Remove the backslash, keep the escaped character
                            result.push(next_char);
                            i += 2;
                        },
                        _ => {
                            // Keep the backslash for other characters
                            result.push(ch);
                            i += 1;
                        }
                    }
                } else {
                    result.push(ch);
                    i += 1;
                }
            },
            _ => {
                result.push(ch);
                i += 1;
            }
        }
    }
    
    result
}

fn find_closing_quote(chars: &[char], start: usize, quote_char: char) -> Option<usize> {
    let mut i = start + 1;
    
    while i < chars.len() {
        if chars[i] == quote_char {
            // Check if it's escaped
            if !is_escaped(chars, i) {
                return Some(i);
            }
        }
        i += 1;
    }
    
    None // No closing quote found
}