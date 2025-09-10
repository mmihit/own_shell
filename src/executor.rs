use crate::command::{ Command, Rm };
use crate::errors::{ CrateResult };
use anyhow::{ anyhow, Ok };
use tokio::fs::{ self, remove_file, read_to_string };
use std::{ result };

pub struct Executor {
    pub current_dir: String,
    pub history: Vec<String>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            current_dir: pwd(),
            history: vec![],
        }
    }

    pub async fn execute(&mut self, command: &Command) -> CrateResult<String> {
        match command {
            Command::Echo(v) => self.echo(v),
            Command::Cd(v) => self.cd(v),
            Command::Ls(ls) => self.ls(ls).await,
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
        Ok(format!("{}\n", input.to_string().replace("\"", "").replace("\'", "")))
    }

    fn cd(&mut self, input: &String) -> CrateResult<String> {
        if let std::result::Result::Ok(()) = std::env::set_current_dir(input.to_string()) {
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
                    res = Err(anyhow!(format!("cannot create directory '{}': {}", path, err)));
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
                result::Result::Err(err) => {
                    match remove_file(&path).await {
                        result::Result::Ok(()) => (),
                        result::Result::Err(_) => {
                            res = Err(anyhow!("cannot remove '{}': {}", path, err));
                        }
                    }
                }
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
                result::Result::Err(err) => res=format!("{}Error: {}: {}\n", res,path, err)
            }
        }
        return Ok(res);
    }

    async fn ls(&self, ls: &crate::command::Ls) -> CrateResult<String> {
        let mut result = String::new();
        
        // Determine which directories to list
        let dirs_to_list = if ls.dirs.is_empty() {
            vec![".".to_string()]
        } else {
            ls.dirs.clone()
        };

        for dir in dirs_to_list {
            let full_path = if dir.starts_with("/") {
                dir.clone()
            } else {
                format!("{}/{}", self.current_dir, dir)
            };

            // Try to read the directory using std::fs (synchronous)
            match std::fs::read_dir(&full_path) {
                std::result::Result::Ok(entries) => {
                    let mut file_names = Vec::new();
                    
                    for entry in entries {
                        match entry {
                            std::result::Result::Ok(entry) => {
                                if let Some(name) = entry.file_name().to_str() {
                                    file_names.push(name.to_string());
                                }
                            }
                            std::result::Result::Err(e) => {
                                return Err(anyhow!("Error reading entry: {}", e));
                            }
                        }
                    }
                    
                    // Sort the file names
                    file_names.sort();
                    
                    // Apply flags
                    match ls.flag.as_str() {
                        "-a" => {
                            // Show all files including hidden ones (starting with .)
                            for name in file_names {
                                result.push_str(&format!("{}\n", name));
                            }
                        }
                        "-l" => {
                            // Long format - for now just show names with basic info
                            for name in file_names {
                                if !name.starts_with('.') { // Skip hidden files for -l
                                    result.push_str(&format!("{}\n", name));
                                }
                            }
                        }
                        "-F" => {
                            // Add indicators for file types
                            for name in file_names {
                                if !name.starts_with('.') { // Skip hidden files for -F
                                    result.push_str(&format!("{}\n", name));
                                }
                            }
                        }
                        _ => {
                            // Default behavior - show non-hidden files
                            for name in file_names {
                                if !name.starts_with('.') {
                                    result.push_str(&format!("{}\n", name));
                                }
                            }
                        }
                    }
                }
                std::result::Result::Err(e) => {
                    return Err(anyhow!("Cannot read directory '{}': {}", dir, e));
                }
            }
        }
        
        Ok(result)
    }
}

fn pwd() -> String {
    let cur_dir = std::env::current_dir().unwrap();
    cur_dir.display().to_string()
}
