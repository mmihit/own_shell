use crate::command::{Command, Rm};
use crate::errors::CrateResult;
use anyhow::{anyhow, Ok};
use std::{env, result, path::Path};
use tokio::fs::{self, read_to_string, remove_file, create_dir_all, remove_dir_all};
 use std::result::Result::Ok as ResultOk;

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

            Command::Ls(ls) => self.ls(ls).await,

            Command::Pwd => self.pwd(),
            Command::Cat(v) => self.cat(v).await,
            Command::Cp(v) => Ok(format!("command: Cp, content: {:?}\n", v)),
            Command::Rm(rm) => self.rm(rm).await,
            Command::Mv(v) => self.mv(v).await,
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

    async fn mv(&self, paths: &Vec<String>) -> CrateResult<String> {
        if paths.len() != 2 {
            return Err(anyhow!("mv requires exactly two arguments: source and destination"));
        }

        let source = &paths[0];
        let dest = &paths[1];

        // Resolve absolute paths
        let source_path = if source.starts_with("/") {
            source.clone()
        } else {
            format!("{}/{}", self.current_dir, source)
        };

        let dest_path = if dest.starts_with("/") {
            dest.clone()
        } else {
            format!("{}/{}", self.current_dir, dest)
        };

        // Check if source exists
        let source_metadata = match fs::metadata(&source_path).await {
            ResultOk(meta) => meta,
            Err(_) => return Err(anyhow!("cannot stat '{}': No such file or directory", source)),
        };

        // Check if destination exists
        let dest_metadata = fs::metadata(&dest_path).await.ok();
        let is_dest_dir = dest_metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

        // Determine final destination path
        let final_dest = if is_dest_dir {
            // If destination is a directory, move source into it
            let source_name = Path::new(source).file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow!("invalid source filename"))?;
            format!("{}/{}", dest_path, source_name)
        } else {
            dest_path.clone()
        };

        // Check if source and destination are the same
        if source_path == final_dest {
            return Ok(String::new()); // No-op
        }

        // Try fast path: rename
        match fs::rename(&source_path, &final_dest).await {
            ResultOk(_) => return Ok(String::new()),
            Err(e) => {
                // If rename fails due to cross-device error, use copy + remove
                if e.raw_os_error() == Some(18) || e.to_string().contains("cross-device") {
                    return self.move_cross_device(&source_path, &final_dest, source_metadata.is_dir()).await;
                }
                return Err(anyhow!("cannot move '{}' to '{}': {}", source, dest, e));
            }
        }
    }

    async fn move_cross_device(&self, source: &str, dest: &str, is_dir: bool) -> CrateResult<String> {
        if is_dir {
            self.copy_directory_recursive(source, dest).await?;
            remove_dir_all(source).await.map_err(|e| anyhow!("failed to remove source directory '{}': {}", source, e))?;
        } else {
            fs::copy(source, dest).await.map_err(|e| anyhow!("failed to copy file '{}' to '{}': {}", source, dest, e))?;
            remove_file(source).await.map_err(|e| anyhow!("failed to remove source file '{}': {}", source, e))?;
        }
        Ok(String::new())
    }

    async fn copy_directory_recursive(&self, source: &str, dest: &str) -> CrateResult<()> {
        use std::collections::VecDeque;
        
        // Create destination directory
        create_dir_all(dest).await.map_err(|e| anyhow!("failed to create directory '{}': {}", dest, e))?;

        // Use a queue for iterative directory traversal
        let mut queue = VecDeque::new();
        queue.push_back((source.to_string(), dest.to_string()));

        while let Some((current_source, current_dest)) = queue.pop_front() {
            // Read current directory
            let mut entries = fs::read_dir(&current_source).await.map_err(|e| anyhow!("failed to read directory '{}': {}", current_source, e))?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| anyhow!("failed to read directory entry: {}", e))? {
                let entry_path = entry.path();
                let entry_name = entry.file_name();
                let dest_path = Path::new(&current_dest).join(&entry_name);

                if entry.metadata().await.map_err(|e| anyhow!("failed to get metadata for '{}': {}", entry_path.display(), e))?.is_dir() {
                    // Create subdirectory and add to queue
                    create_dir_all(&dest_path).await.map_err(|e| anyhow!("failed to create directory '{}': {}", dest_path.display(), e))?;
                    queue.push_back((
                        entry_path.to_str().ok_or_else(|| anyhow!("invalid path"))?.to_string(),
                        dest_path.to_str().ok_or_else(|| anyhow!("invalid path"))?.to_string()
                    ));
                } else {
                    // Copy file
                    fs::copy(&entry_path, &dest_path).await.map_err(|e| anyhow!("failed to copy file '{}' to '{}': {}", entry_path.display(), dest_path.display(), e))?;
                }
            }
        }

        Ok(())
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
