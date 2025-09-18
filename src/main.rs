use anyhow::Result;
use tokio::io::{ self, AsyncBufReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;

mod command;
mod errors;
mod executor;
mod helpers;
use command::Command;
use errors::CrateResult;
use executor::Executor;
use helpers::handle_quotes;

fn spawn_user_input_handle() -> JoinHandle<CrateResult<()>> {
    tokio::spawn(async {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = io::BufReader::new(stdin).lines();
        let mut stdout = io::BufWriter::new(stdout);
        let mut executor = Executor::new();
        let mut is_ctrl_d = true;
        stdout.write(
            format!("Hello to my own shell programm:\n> {}$ ", executor.current_dir).as_bytes()
        ).await?;
        stdout.flush().await?;
        while let Ok(Some(line)) = reader.next_line().await {
            let input = line.as_str();
            // Get the complete input with closed quotes
            let complete_input = match handle_quotes(input, &mut stdout).await {
                Ok(complete) => complete,
                Err(e) => {
                    stdout.write(format!("Error reading input: {}\n", e).as_bytes()).await?;
                    continue;
                }
            };
            match complete_input.as_str() {
                "" => (),
                processed_input =>
                    match Command::try_from(processed_input) {
                        Ok(command) =>
                            match executor.execute(&command).await {
                                anyhow::Result::Ok(res) => {
                                    stdout.write(res.as_bytes()).await?;
                                    if command == Command::Exit {
                                        is_ctrl_d = false;
                                        break;
                                    }
                                }
                                anyhow::Result::Err(error) => {
                                    stdout.write(format!("Error: {}\n", error).as_bytes()).await?;
                                }
                            }
                        Err(err) => {
                            stdout.write(format!("Error: {}\n", err).as_bytes()).await?;
                        }
                    }
            }
            stdout.write(format!("> {}$ ", executor.current_dir).as_bytes()).await?;
            stdout.flush().await?;
        }
        if is_ctrl_d {
            stdout.write(b"\n").await?;
        }

        stdout.flush().await?;
        Ok(())
    })
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    spawn_user_input_handle().await?
}
