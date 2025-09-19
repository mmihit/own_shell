use std::fs;
use std::io;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::time::SystemTime;
use chrono::{ DateTime, Local };
use users::{ get_user_by_uid, get_group_by_gid };

pub fn ls(args: &[String]) -> io::Result<()> {
    let mut show_hidden = false;
    let mut long_format = false;
    let mut classify = false;
    let mut paths = Vec::new();

    // Parse command line arguments
    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
            for ch in arg.chars().skip(1) {
                match ch {
                    'a' => {
                        show_hidden = true;
                    }
                    'l' => {
                        long_format = true;
                    }
                    'F' => {
                        classify = true;
                    }
                    _ => {
                        eprintln!("ls: invalid option -- '{}'", ch);
                        return Ok(());
                    }
                }
            }
        } else {
            paths.push(arg.to_string());
        }
    }

    // If no paths specified, use current directory
    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let mut first_path = true;
    for path in &paths {

        if paths.len() > 1 {
            if !first_path {
                println!();
            }
            println!("{}:", path);
        }
        first_path = false;

        // Check if path exists
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(e) => {
                eprintln!("ls: cannot access '{}': {}", path, e);
                continue;
            }
        };
        if !metadata.is_dir() {
            if long_format {
                let file_name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                print_long_format(&metadata, &file_name, classify, path)?;
            } else {
                let mut display_name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());

                if classify {
                    display_name = add_file_type_indicator(&metadata, display_name);
                }
                println!("{}", display_name);
            }
            continue;
        }


        // Process to list all entries
        list_directory(path, show_hidden, long_format, classify)?;
    }

    Ok(())
}

fn list_directory(path: &str, show_hidden: bool, long_format: bool, classify: bool) -> io::Result<()> {
    let entries = fs::read_dir(path)?;
    let mut files: Vec<_> = entries.collect::<Result<Vec<_>, _>>()?;

    // Filter hidden files if -a not specified
    if !show_hidden {
        files.retain(|entry| { !entry.file_name().to_string_lossy().starts_with('.') });
    }

    let mut special_entries = Vec::new();
    if show_hidden {
        let current_path = if path == "." { "." } else { &format!("{}/.", path) };
        let parent_path = if path == "." { ".." } else { &format!("{}/..", path) };

        if let Ok(current_meta) = fs::metadata(current_path) {
            special_entries.push((".".to_string(), current_meta));
        }
        if let Ok(parent_meta) = fs::metadata(parent_path) {
            special_entries.push(("..".to_string(), parent_meta));
        }
    }

    // Sort entries by name
    files.sort_by(|a, b| {
        let name_a = a.file_name().to_string_lossy().to_string().to_lowercase();
        let name_b = b.file_name().to_string_lossy().to_string().to_lowercase();
        // for sort the hidden file too
        let clean_a = name_a.strip_prefix('.').unwrap_or(&name_a);
        let clean_b = name_b.strip_prefix('.').unwrap_or(&name_b);
        clean_a.cmp(clean_b)
    });

    if long_format {
        let mut total_blocks = 0u64;
        for (_, metadata) in &special_entries {
            total_blocks += get_blocks(metadata);
        }
        for entry in &files {
            let metadata = entry.metadata()?;
            total_blocks += get_blocks(&metadata);
        }
        println!("total {}", total_blocks);

        // Display special entries first (. and ..)
        for (name, metadata) in special_entries {
            print_long_format(&metadata, &name, classify, path)?;
        }

        // Display entries
        for entry in files {
            let metadata = entry.metadata()?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            print_long_format(&metadata, &file_name_str, classify, path)?;
        }
    } else {
        let mut display_names = Vec::new();

        // Add special entries
        for (name, metadata) in special_entries {
            let mut display_name = name;
            if classify {
                display_name = add_file_type_indicator(&metadata, display_name);
            }
            display_names.push(display_name);
        }

        // Add regular entries
        for entry in files {
            let metadata = entry.metadata()?;
            let file_name_str = entry.file_name().to_string_lossy().to_string();
            let mut display_name = file_name_str;

            if classify {
                display_name = add_file_type_indicator(&metadata, display_name);
            }
            display_names.push(display_name);
        }

        print_in_columns(&display_names);
    }

    Ok(())
}

fn print_in_columns(names: &[String]) {
    if names.is_empty() {
        return;
    }
    let terminal_width = get_terminal_width().unwrap_or(80);
    let max_len = names.iter().map(|name| name.len()).max().unwrap_or(0);
    let col_width = max_len + 2;
    let num_cols = std::cmp::max(1, terminal_width / col_width);
    // Calculate number of rows needed
    let num_rows = (names.len() + num_cols - 1) / num_cols;
    for row in 0..num_rows {
        for col in 0..num_cols {
            let index = col * num_rows + row;
            if index < names.len() {
                let name = &names[index];
                if col < num_cols - 1 && index + num_rows < names.len() {
                    // Not the last column and not the last item, pad to column width
                    print!("{:<width$}", name, width = col_width);
                } else {
                    // Last column or last item, no padding needed
                    print!("{}", name);
                }
            }
        }
        println!();
    }
}

fn get_terminal_width() -> Option<usize> {
    #[cfg(unix)]
    {
        if let Ok(width_str) = std::env::var("COLUMNS") {
            if let Ok(width) = width_str.parse::<usize>() {
                return Some(width);
            }
        }
    }
    // Fallback to reasonable default
    Some(80)
}

#[cfg(unix)]
fn get_blocks(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    (metadata.blocks() + 1) / 2
}

fn print_long_format(metadata: &fs::Metadata, file_name: &str, classify: bool, base_path: &str) -> io::Result<()> {
    // File type and permissions
    let file_type = get_file_type(metadata);
    let extended_permission = format!("{}/{}",base_path, file_name);
    // println!("{}", extended_permission);
    let permissions = format_permissions(metadata, &extended_permission);

    // Number of hard links
    let nlinks = get_nlinks(metadata);

    // Owner and group (simplified - showing as numbers on Unix)
    let (uid, gid) = get_owner_info(metadata);

    let username = get_user_by_uid(uid)
        .map(|u| u.name().to_string_lossy().to_string())
        .unwrap_or_else(|| uid.to_string());

    let groupname = get_group_by_gid(gid)
        .map(|g| g.name().to_string_lossy().to_string())
        .unwrap_or_else(|| gid.to_string());

    // File size
    let size = metadata.len();

    // Modification time
    let mtime = format_ls_time(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH));

    let mut display_name = if classify {
        add_file_type_indicator(metadata, file_name.to_string())
    } else {
        file_name.to_string()
    };

    // Handle symbolic links - show target
    if metadata.is_symlink() {
        // Construct the full path for the symlink
        let full_path = if base_path == "." {
            file_name.to_string()
        } else {
            // Check if base_path is actually pointing to the same file as file_name
            // This handles cases like 'ls -l /bin' where /bin might be a symlink itself
            let base_path_obj = std::path::Path::new(base_path);
            let file_name_obj = std::path::Path::new(file_name);

            if base_path_obj.file_name() == Some(file_name_obj.as_os_str()) {
                base_path.to_string()
            } else {
                format!("{}/{}", base_path.trim_end_matches('/'), file_name)
            }
        };

        if let Ok(target) = fs::read_link(&full_path) {
            let link_name = file_name.to_string();

            if classify {
                let target_path = if target.is_absolute() {
                    target.clone()
                } else {
                    let symlink_dir = std::path::Path
                        ::new(&full_path)
                        .parent()
                        .unwrap_or(std::path::Path::new("."));
                    symlink_dir.join(&target)
                };

                if let Ok(target_metadata) = fs::metadata(&target_path) {
                    let target_display = add_file_type_indicator(
                        &target_metadata,
                        target.display().to_string()
                    );
                    display_name = format!("{} -> {}", link_name, target_display);
                } else {
                    display_name = format!("{} -> {}", link_name, target.display());
                }
            } else {
                display_name = format!("{} -> {}", file_name, target.display());
            }
        } else {
            display_name = if classify {
                add_file_type_indicator(metadata, file_name.to_string())
            } else {
                file_name.to_string()
            };
        }
    }

    if metadata.file_type().is_char_device() || metadata.file_type().is_block_device() {
        let dev_id = metadata.rdev();
        let major = { libc::major(dev_id) };
        let minor = { libc::minor(dev_id) };
        println!(
            "{}{:<11} {:>3} {:<8} {:<8} {:>3}, {:>5} {} {}",
            file_type, permissions, nlinks, username, groupname, major, minor, mtime, display_name
        );
    } else {
        println!(
            "{}{:<11} {:>3} {:<8} {:<8} {:>10} {} {}",
            file_type, permissions, nlinks, username, groupname, size, mtime, display_name
        );
    }

    Ok(())
}

fn get_file_type(metadata: &fs::Metadata) -> char {
    use std::os::unix::fs::FileTypeExt;

    let file_type = metadata.file_type();

    if file_type.is_dir() {
        'd'
    } else if file_type.is_symlink() {
        'l'
    } else if file_type.is_char_device() {
        'c'
    } else if file_type.is_block_device() {
        'b'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_socket() {
        's'
    } else {
        '-'
    }
}

fn format_permissions(metadata: &fs::Metadata, file_path: &str) -> String {
    let mode = metadata.permissions().mode();

    let mut perms = String::new();

    // Owner permissions
    perms.push(if (mode & 0o400) != 0 { 'r' } else { '-' });
    perms.push(if (mode & 0o200) != 0 { 'w' } else { '-' });
    // perms.push(if (mode & 0o100) != 0 { 'x' } else { '-' });
    if (mode & 0o100) != 0 {
        if (mode & 0o4000) != 0 {
            perms.push('s'); // setuid + execute
        } else {
            perms.push('x');
        }
    } else {
        if (mode & 0o4000) != 0 {
            perms.push('S'); // setuid without execute
        } else {
            perms.push('-');
        }
    }

    // Group permissions
    perms.push(if (mode & 0o040) != 0 { 'r' } else { '-' });
    perms.push(if (mode & 0o020) != 0 { 'w' } else { '-' });
    perms.push(if (mode & 0o010) != 0 { 'x' } else { '-' });

    // Other permissions
    perms.push(if (mode & 0o004) != 0 { 'r' } else { '-' });
    perms.push(if (mode & 0o002) != 0 { 'w' } else { '-' });
    // perms.push(if (mode & 0o001) != 0 { 'x' } else { '-' });
    if (mode & 0o001) != 0 {
    if (mode & 0o1000) != 0 {
        perms.push('t'); // sticky + execute
        } else {
            perms.push('x');
        }
    } else {
    if (mode & 0o1000) != 0 {
        perms.push('T'); // sticky without execute
        } else {
            perms.push('-');
        }
    }

    if has_extended_attributes(file_path) {
        perms.push('+');
    }

    perms
}

#[cfg(unix)]
fn get_nlinks(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink()
}

#[cfg(unix)]
fn get_owner_info(metadata: &fs::Metadata) -> (u32, u32) {
    use std::os::unix::fs::MetadataExt;
    (metadata.uid(), metadata.gid())
}

fn format_ls_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%b %e %H:%M").to_string()
}

fn add_file_type_indicator(metadata: &fs::Metadata, mut name: String) -> String {
    if metadata.is_dir() {
        name.push('/');
    } else if metadata.is_symlink() {
        name.push('@');
    } else if is_executable(metadata) {
        name.push('*');
    } else if metadata.file_type().is_socket() {
        name.push('=');
    } else if metadata.file_type().is_fifo() {
        name.push('|');
    } 
    name
}

fn is_executable(metadata: &fs::Metadata) -> bool {
    let mode = metadata.permissions().mode();
    (mode & 0o111) != 0
}

fn has_extended_attributes(file_path: &str) -> bool {
    if let Ok(metadata) = fs::symlink_metadata(file_path) {
        let file_type = metadata.file_type();
        if !file_type.is_char_device() && !file_type.is_block_device() {
            return false;
        }
    } else {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        if let Ok(c_path) = CString::new(file_path) {
            unsafe {
                // Check specifically for POSIX ACL extended attributes
                use std::ptr;
                let acl_access = CString::new("system.posix_acl_access").unwrap();
                
                // Check for access ACLs
                let has_access_acl = libc::getxattr(
                    c_path.as_ptr(),
                    acl_access.as_ptr(),
                    ptr::null_mut(),
                    0
                ) > 0;
                
                has_access_acl
            }
        } else {
            false
        }
    }
}