use crate::command::Command;
use crate::errors::{CrateResult};
use anyhow::{anyhow, Ok};

pub struct Executor {
    pub current_dir: String,
    pub history: Vec<String>
}


impl Executor {
    pub fn new() -> Self {
        Self{
        current_dir: pwd(),
        history: vec![]
        }
    }

    pub async fn execute(&mut self, command: &Command) -> CrateResult<String> {
        match command {
            Command::Echo(v) => self.echo(v),
            Command::Cd(v) => self.cd(v),
            Command::Ls(ls) => Ok(format!("command: Ls, content: {:?}, flag: {}\n", ls.dirs, ls.flag)),
            Command::Pwd => self.pwd(),
            Command::Cat(v) => Ok(format!("command: Cat, content: {:?}\n", v)),
            Command::Cp(v) => Ok(format!("command: Cp, content: {:?}\n", v)),
            Command::Rm(rm) => Ok(format!("command: Rm, is directory: {}, content: {:?}\n", rm.is_dir, rm.dirs)),
            Command::Mv(v) => Ok(format!("command: Mv, content: {:?}\n", v)),
            Command::Mkdir(v) => Ok(format!("command: Mkdir, content: {:?}\n", v)),
            Command::Exit => self.exist()
        }
    }
    pub fn pwd(&self) -> CrateResult<String> {
        Ok(format!("{}\n",pwd()))
    }
    fn exist(&self) -> CrateResult<String> {
        Ok(String::new())
    }
    fn echo(&self, input: &String) -> CrateResult<String> {
        Ok(format!("{}\n",input.to_string().replace("\"", "").replace("\'", "")))
    }

    fn cd(&mut self, input: &String) -> CrateResult<String> {
        if let std::result::Result::Ok(())=std::env::set_current_dir(input.to_string()) {
            self.current_dir=pwd();
            Ok(String::new())
        }else {
            Err(anyhow!("no such file or directory: {}",input))
        }
    }

    //...
}

fn pwd() -> String {
        let cur_dir = std::env::current_dir().unwrap();
        cur_dir.display().to_string()
}