use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufWriter, Stdout};
use tokio::task::JoinHandle;

mod command;
mod errors;
mod executor;
use command::Command;
use errors::CrateResult;
use executor::Executor;

fn spawn_user_input_handle() -> JoinHandle<CrateResult<()>> {
    tokio::spawn(async {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = io::BufReader::new(stdin).lines();
        let mut stdout = io::BufWriter::new(stdout);
        let mut executor = Executor::new();
        let mut is_ctrl_d = true;
        stdout
            .write(
                format!(
                    "Hello to my own shell programm:\n> {}$ ",
                    executor.current_dir
                )
                .as_bytes(),
            )
            .await?;
        stdout.flush().await?;
        while let Ok(Some(line)) = reader.next_line().await {
            let input = line.as_str();

            // Get the complete input with closed quotes
            let complete_input = match handle_quotes(input, &mut stdout).await {
                Ok(complete) => complete,
                Err(e) => {
                    stdout
                        .write(format!("Error reading input: {}\n", e).as_bytes())
                        .await?;
                    continue;
                }
            };

            match complete_input.replace("  ", " ").as_str() {
                "" => (),
                processed_input => match Command::try_from(processed_input) {
                    Ok(command) => match executor.execute(&command).await {
                        anyhow::Result::Ok(res) => {
                            stdout.write(res.as_bytes()).await?;
                            if command == Command::Exit {
                                is_ctrl_d = false;
                                break;
                            }
                        }
                        anyhow::Result::Err(error) => {
                            stdout
                                .write(format!("Error: {}\n", error).as_bytes())
                                .await?;
                        }
                    },
                    Err(err) => {
                        stdout.write(format!("Error: {}\n", err).as_bytes()).await?;
                    }
                },
            }
            stdout
                .write(format!("> {}$ ", executor.current_dir).as_bytes())
                .await?;
            stdout.flush().await?;
        }
        if is_ctrl_d {
            stdout.write(b"\n").await?;
        }
        stdout.write(b"Exiting...\n").await?;
        stdout.flush().await?;
        Ok(())
    })
}

async fn handle_quotes(input: &str, stdout: &mut BufWriter<Stdout>) -> io::Result<String> {
    let mut final_input = input.to_string();

    loop {
        let quote_status = check_quotes(&final_input);

        match quote_status {
            QuoteStatus::Balanced => break,
            QuoteStatus::UnclosedSingle | QuoteStatus::UnclosedDouble => {
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
    Ok(final_input)
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
                if !is_escaped(&chars, i) {
                    single_quote_count += 1;
                }
            }
            '"' => {
                if !is_escaped(&chars, i) {
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

    // Count consecutive backslashes before current position
    while pos > 0 && chars[pos - 1] == '\\' {
        escape_count += 1;
        pos -= 1;
    }

    // If odd number of backslashes, the quote is escaped
    escape_count % 2 != 0
}

#[tokio::main]
async fn main() {
    let input_handler = spawn_user_input_handle().await;
    if let Ok(Err(e)) = input_handler {
        eprintln!("Error: {}", e)
    }
}
