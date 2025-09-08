use crate::command::{Command, Rm};
use crate::errors::CrateResult;
use anyhow::{anyhow, Ok};
use std::{env, result};
use tokio::fs::{self, read_to_string, remove_file};

pub struct Executor {
    pub current_dir: String,
    pub _history: Vec<String>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            current_dir: pwd(),
            _history: vec![],
        }
    }

    pub async fn execute(&mut self, command: &Command) -> CrateResult<String> {
        match command {
            Command::Echo(v) => self.echo(v),
            Command::Cd(v) => self.cd(v),
            Command::Ls(ls) => Ok(format!(
                "command: Ls, content: {:?}, flag: {}\n",
                ls.dirs, ls.flag
            )),
            Command::Pwd => self.pwd(),
            Command::Cat(v) => self.cat(v).await,
            Command::Cp(v) => Ok(format!("command: Cp, content: {:?}\n", v)),
            Command::Rm(rm) => self.rm(rm).await,
            Command::Mv(v) => Ok(format!("command: Mv, content: {:?}\n", v)),
            Command::Mkdir(v) => self.mkdir(v).await,
            Command::Exit => self.exit(),
        }
    }
    pub fn pwd(&self) -> CrateResult<String> {
        Ok(format!("{}\n", pwd()))
    }
    fn exit(&self) -> CrateResult<String> {
        Ok(String::new())
    }
    fn echo(&self, input: &String) -> CrateResult<String> {
        Ok(format!(
            "{}\n",
            input.to_string().replace("\"", "").replace("\'", "")
        ))
    }

    fn cd(&mut self, input:  &String) -> CrateResult<String> {
        let mut input = input.clone();
        if input.len() == 0 {
            if let Some(home_path) = env::home_dir() {
                input =  home_path.to_str().unwrap().to_string();
            } else {
                return Err(anyhow!("something wrong, please fix it!..\n"))
            }
        } 
        if let std::result::Result::Ok(()) = std::env::set_current_dir(&input) {
            self.current_dir = pwd();
            Ok(String::new())
        } else {
            Err(anyhow!("no such file or directory: {}", input))
        }
    }

    async fn mkdir(&mut self, input: &Vec<String>) -> CrateResult<String> {
        let mut res = Ok(String::new());
        for path in input.iter() {
            let full_path: String = if path.starts_with("/") {
                path.to_string()
            } else {
                format!("{}/{}", self.current_dir, path)
            };
            match fs::create_dir(&full_path).await {
                result::Result::Ok(()) => (),
                result::Result::Err(err) => {
                    res = Err(anyhow!(format!(
                        "cannot create directory '{}': {}",
                        path, err
                    )));
                }
            }
        }
        return res;
    }

    async fn rm(&mut self, input: &Rm) -> CrateResult<String> {
        let mut res = Ok(String::new());
        for path in input.dirs.iter() {
            let full_path: String = if path.starts_with("/") {
                path.to_string()
            } else {
                format!("{}/{}", self.current_dir, path)
            };

            let action = if !input.is_dir {
                fs::remove_file(&full_path).await
            } else {
                fs::remove_dir_all(&full_path).await
            };

            match action {
                result::Result::Ok(()) => (),
                result::Result::Err(err) => match remove_file(&path).await {
                    result::Result::Ok(()) => (),
                    result::Result::Err(_) => {
                        res = Err(anyhow!("cannot remove '{}': {}", path, err));
                    }
                },
            }
        }
        return res;
    }

    async fn cat(&self, input: &Vec<String>) -> CrateResult<String> {
        let mut res = String::new();
        for path in input.iter() {
            let full_path: String = if path.starts_with("/") {
                path.to_string()
            } else {
                format!("{}/{}", self.current_dir, path)
            };
            match read_to_string(full_path).await {
                result::Result::Ok(content) => res.push_str(&(content)),
                result::Result::Err(err) => res = format!("{}Error: {}: {}\n", res, path, err),
            }
        }
        return Ok(res);
    }
}

fn pwd() -> String {
    let cur_dir = std::env::current_dir().unwrap();
    cur_dir.display().to_string()
}
