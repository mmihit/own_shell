use crate::command::{ Command, Rm };
use crate::errors::CrateResult;
use anyhow::{ anyhow, Ok };
use std::result::Result::Ok as ResultOk;
use std::{ path::Path, result };
use crate::helpers::{collect_data, pwd, display_ls_result};
use tokio::fs::{ self, create_dir_all, read_to_string, remove_dir_all, remove_file };
// use crathelpers::pwd;

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
            Command::Cp(v) => self.cp(v).await,
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
        Ok(format!("{}\n", input))
    }

    fn cd(&mut self, input: &String) -> CrateResult<String> {
        let mut input = input.clone();
        if input.len() == 0 {
            if let Some(home_path) = dirs::home_dir() {
                input = home_path.to_str().unwrap().to_string();
            } else {
                return Err(anyhow!("something wrong, please fix it!..\n"));
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
                result::Result::Err(err) =>
                    match remove_file(&path).await {
                        result::Result::Ok(()) => (),
                        result::Result::Err(_) => {
                            res = Err(anyhow!("cannot remove '{}': {}", path, err));
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
                result::Result::Ok(content) => res.push_str(&content),
                result::Result::Err(err) => {
                    res = format!("{}Error: {}: {}\n", res, path, err);
                }
            }
        }
        return Ok(res);
    }

    async fn cp(&self, input: &Vec<String>) -> CrateResult<String> {
        let sources: Vec<String> = input[..input.len() - 1].to_vec();
        let last_index = &input[input.len() - 1];
        let destination = if last_index.starts_with("/") {
            last_index.clone()
        } else {
            format!("{}/{}", self.current_dir, last_index)
        };

        let metadata_result = tokio::fs::metadata(&destination).await;
        let mut is_destination_not_exist: bool = false;
        let mut is_destination_file = false;
        match metadata_result {
            Err(error) => {
                if sources.len() > 1 {
                    return Err(anyhow!(error));
                } else {
                    is_destination_not_exist = true;
                }
            }
            result::Result::Ok(metadata) => {
                if metadata.is_file() {
                    is_destination_file = true;
                }
                if sources.len() > 1 && is_destination_file {
                    return Err(anyhow!("The destination should be a directory"));
                }
            }
        }

        // Get metadata again for later use

        for s in &sources {
            if s == &destination {
                return Err(
                    anyhow!(
                        format!("The \"{}\" can't be the source and the destination at the same time", s)
                    )
                );
            }
        }

        for s in sources {
            let new_file_name = if is_destination_file || is_destination_not_exist {
                destination.to_string()
            } else {
                let filename = Path::new(&s)
                    .file_name()
                    .ok_or_else(|| anyhow!("Invalid source path"))?;
                Path::new(&destination).join(filename).to_string_lossy().to_string()
            };

            let copy_result = tokio::fs::copy(s, new_file_name).await;
            match copy_result {
                result::Result::Ok(_) => (),
                Err(error) => {
                    return Err(anyhow!(error));
                }
            }
        }
        Ok(String::new())
    }

    async fn mv(&self, paths: &Vec<String>) -> CrateResult<String> {
        if paths.len() < 2 {
            return Err(anyhow!("mv requires at least two arguments: ccsource(s) and destination"));
        }

        // The last argument is the destination
        let dest = &paths[paths.len() - 1];
        let sources = &paths[..paths.len() - 1];

        // Resolve absolute path for destination
        let dest_path = if dest.starts_with("/") {
            dest.clone()
        } else {
            format!("{}/{}", self.current_dir, dest)
        };

        // Check if destination exists and is a directory
        let dest_metadata = fs::metadata(&dest_path).await.ok();
        let is_dest_dir = dest_metadata
            .as_ref()
            .map(|m| m.is_dir())
            .unwrap_or(false);

        // If we have multiple sources, destination must be a directory
        if sources.len() > 1 && dest_metadata.is_some() && !is_dest_dir {
            return Err(
                anyhow!("cannot move multiple files: destination '{}' is not a directory", dest)
            );
        }

        // Move each source to the destination
        for source in sources {
            // Resolve absolute path for source
            let source_path = if source.starts_with("/") {
                source.clone()
            } else {
                format!("{}/{}", self.current_dir, source)
            };

            // Check if source exists
            let source_metadata = match fs::metadata(&source_path).await {
                ResultOk(meta) => meta,
                Err(_) => {
                    eprintln!("cannot stat '{}': No such file or directory", source);
                    continue; // Skip this source and continue with others
                }
            };

            // Determine final destination path
            let final_dest = if is_dest_dir {
                // If destination is a directory, move source into it
                let source_name = Path::new(source)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow!("invalid source filename"))?;
                format!("{}/{}", dest_path, source_name)
            } else {
                dest_path.clone()
            };

            // Check if source and destination are the same
            if source_path == final_dest {
                continue; // No-op, skip this source
            }

            // Try fast path: rename
            match fs::rename(&source_path, &final_dest).await {
                ResultOk(_) => {
                    continue;
                } // Success, continue with next source
                Err(e) => {
                    // If rename fails due to cross-device error, use copy + remove
                    if e.raw_os_error() == Some(18) || e.to_string().contains("cross-device") {
                        if
                            let Err(cross_err) = self.move_cross_device(
                                &source_path,
                                &final_dest,
                                source_metadata.is_dir()
                            ).await
                        {
                            eprintln!("cannot move '{}' to '{}': {}", source, dest, cross_err);
                        }
                        continue;
                    }
                    eprintln!("cannot move '{}' to '{}': {}", source, dest, e);
                    continue; // Continue with other sources even if one fails
                }
            }
        }

        Ok(String::new())
    }

    async fn move_cross_device(
        &self,
        source: &str,
        dest: &str,
        is_dir: bool
    ) -> CrateResult<String> {
        if is_dir {
            self.copy_directory_recursive(source, dest).await?;
            remove_dir_all(source).await.map_err(|e|
                anyhow!("failed to remove source directory '{}': {}", source, e)
            )?;
        } else {
            fs
                ::copy(source, dest).await
                .map_err(|e| anyhow!("failed to copy file '{}' to '{}': {}", source, dest, e))?;
            remove_file(source).await.map_err(|e|
                anyhow!("failed to remove source file '{}': {}", source, e)
            )?;
        }
        Ok(String::new())
    }

    async fn copy_directory_recursive(&self, source: &str, dest: &str) -> CrateResult<()> {
        use std::collections::VecDeque;

        // create destination directory
        create_dir_all(dest).await.map_err(|e|
            anyhow!("failed to create directory '{}': {}", dest, e)
        )?;

        // use a queue for iterative directory traversal
        let mut queue = VecDeque::new();
        queue.push_back((source.to_string(), dest.to_string()));

        while let Some((current_source, current_dest)) = queue.pop_front() {
            // read current directory
            let mut entries = fs
                ::read_dir(&current_source).await
                .map_err(|e| anyhow!("failed to read directory '{}': {}", current_source, e))?;

            while
                let Some(entry) = entries
                    .next_entry().await
                    .map_err(|e| anyhow!("failed to read directory entry: {}", e))?
            {
                let entry_path = entry.path();
                let entry_name = entry.file_name();
                let dest_path = Path::new(&current_dest).join(&entry_name);

                if
                    entry
                        .metadata().await
                        .map_err(|e|
                            anyhow!("failed to get metadata for '{}': {}", entry_path.display(), e)
                        )?
                        .is_dir()
                {
                    // create subdirectory and add to queue
                    create_dir_all(&dest_path).await.map_err(|e|
                        anyhow!("failed to create directory '{}': {}", dest_path.display(), e)
                    )?;
                    queue.push_back((
                        entry_path
                            .to_str()
                            .ok_or_else(|| anyhow!("invalid path"))?
                            .to_string(),
                        dest_path
                            .to_str()
                            .ok_or_else(|| anyhow!("invalid path"))?
                            .to_string(),
                    ));
                } else {
                    // copy file
                    fs
                        ::copy(&entry_path, &dest_path).await
                        .map_err(|e|
                            anyhow!(
                                "failed to copy file '{}' to '{}': {}",
                                entry_path.display(),
                                dest_path.display(),
                                e
                            )
                        )?;
                }
            }
        }

        Ok(())
    }

    async fn ls(&self, ls: &crate::command::Ls) -> CrateResult<String> {
        // let directories = collect_data(ls.is_all, ls.is_classify, ls.is_listing, ls.dirs.clone());
        match collect_data(ls.is_all, ls.is_classify, ls.is_listing, ls.dirs.clone()) {
            anyhow::Result::Ok(data) => Ok(display_ls_result(ls.is_all, ls.is_classify, ls.is_listing, data)),
            anyhow::Result::Err(err) => Err(err),
        }
    }
}
