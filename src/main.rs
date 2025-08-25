use tokio::io::{ self, AsyncBufReadExt, AsyncWriteExt };
use tokio::task::JoinHandle;

mod command;
mod executor;
mod errors;
use executor::Executor;
use command::Command;
use errors::CrateResult;

fn spawn_user_iput_handle() -> JoinHandle<CrateResult<()>> {
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
            let input = line.as_str().trim();
            match input {
                "" => (),
                _ =>
                    match Command::try_from(input) {
                        Ok(command) =>
                            match executor.execute(&command).await {
                                anyhow::Result::Ok(res) => {
                                    stdout.write(res.as_bytes()).await?;
                                    if command == Command::Exit {
                                        is_ctrl_d = false;
                                        break;
                                    }
                                },
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
        stdout.write(b"Exiting...\n").await?;
        stdout.flush().await?;
        Ok(())
    })
}

#[tokio::main]
async fn main() {
    let input_handler = spawn_user_iput_handle().await;
    if let Ok(Err(e)) = input_handler {
        eprintln!("Error: {}", e)
    }
}
