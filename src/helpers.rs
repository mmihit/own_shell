use anyhow::{ anyhow };
use tokio::io::{ self, AsyncBufReadExt, AsyncWriteExt, BufWriter, Stdout };
use std::fs;
use std::path::{ Path };
use crate::errors::CrateResult;
use chrono::Datelike;
use std::os::unix::fs::MetadataExt;

#[derive(Debug, PartialEq)]
enum QuoteStatus {
    Balanced,
    UnclosedSingle,
    UnclosedDouble,
}

pub async fn handle_quotes(input: &str, stdout: &mut BufWriter<Stdout>) -> io::Result<Vec<String>> {
    let mut final_input = input.to_string();

    loop {
        let quote_status = check_quotes(&final_input);

        match quote_status {
            QuoteStatus::Balanced => {
                break;
            }
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

    Ok(process_shell_quotes(&final_input))
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
                if !is_escaped(&chars, i) && single_quote_count == 0 {
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

fn process_shell_quotes(input: &str) -> Vec<String> {
    if input.is_empty() {
        return vec![];
    }

    let chars: Vec<char> = input.chars().collect();
    let mut result = Vec::new();
    let mut current_token = String::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        match ch {
            '"' => {
                // Find the closing double quote
                if let Some(closing_pos) = find_closing_quote(&chars, i, '"') {
                    // Add content between quotes (preserving spaces)
                    for j in i + 1..closing_pos {
                        current_token.push(chars[j]);
                    }
                    i = closing_pos + 1;
                } else {
                    // No closing quote, treat as literal
                    current_token.push(ch);
                    i += 1;
                }
            }
            '\'' => {
                // Find the closing single quote
                if let Some(closing_pos) = find_closing_quote(&chars, i, '\'') {
                    // Add content between quotes (preserving spaces)
                    for j in i + 1..closing_pos {
                        current_token.push(chars[j]);
                    }
                    i = closing_pos + 1;
                } else {
                    // No closing quote, treat as literal
                    current_token.push(ch);
                    i += 1;
                }
            }
            '\\' => {
                // Handle escaped characters
                if i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    match next_char {
                        '"' | '\'' | '\\' => {
                            current_token.push(next_char);
                            i += 2;
                        }
                        _ => {
                            current_token.push(ch);
                            i += 1;
                        }
                    }
                } else {
                    current_token.push(ch);
                    i += 1;
                }
            }
            ' ' => {
                // Space outside quotes - end current token
                if !current_token.is_empty() {
                    result.push(current_token.clone());
                    current_token.clear();
                }
                // Skip multiple consecutive spaces
                while i + 1 < chars.len() && chars[i + 1] == ' ' {
                    i += 1;
                }
                i += 1;
            }
            _ => {
                current_token.push(ch);
                i += 1;
            }
        }
    }

    // Add the last token if it exists
    if !current_token.is_empty() {
        result.push(current_token);
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

    None
}

#[derive(Debug, Clone, Default)]
pub struct FileInfo {
    pub name: String,
    pub r#type: String,
    pub full_path: String,
    pub permissions: Vec<String>,
    pub user: String,
    pub group: String,
    pub permission_bits: usize,
    pub device_info: (u64, u64),
    pub symlink_target: Option<String>,
    pub links: u64,
    pub size: u64,
    pub modified_time: String,
}

#[derive(Debug, Clone)]
pub struct Directory {
    pub name: String,
    pub file_content: Vec<FileInfo>,
}

pub fn collect_data(
    is_all: bool,
    _is_classify: bool,
    _is_listing: bool,
    dirs: Vec<String>
) -> CrateResult<Vec<Directory>> {
    let mut results: Vec<Directory> = Vec::new();

    for dir in dirs {
        let display_name = &dir;
        let current_path = pwd();
        let target_dir_path = join_path(&current_path, &dir);

        let mut entries: Vec<FileInfo> = Vec::new();

        match fs::metadata(&target_dir_path) {
            Ok(md) if md.is_file() => {
                let name = Path::new(&dir)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&dir)
                    .to_string();
                let mut info = FileInfo {
                    name,
                    r#type: String::from("file"),
                    full_path: (&target_dir_path).to_string(),
                    ..Default::default()
                };
                if _is_listing {
                    populate_listing_info(&mut info);
                }
                entries.push(info);
            }
            Ok(md) if md.is_dir() => {
                // When listing a specific directory (not "."), include '.' and '..' entries first
                if is_all {
                    let mut dot = FileInfo {
                        name: String::from("."),
                        r#type: String::from("directory"),
                        full_path: (&target_dir_path).to_string(),
                        ..Default::default()
                    };
                    if _is_listing {
                        populate_listing_info(&mut dot);
                    }
                    entries.push(dot);

                    let mut dotdot = FileInfo {
                        name: String::from(".."),
                        r#type: String::from("directory"),
                        full_path: join_path(&target_dir_path.to_string(), ".."),
                        ..Default::default()
                    };
                    if _is_listing {
                        populate_listing_info(&mut dotdot);
                    }
                    entries.push(dotdot);
                }

                if let Ok(read_dir) = fs::read_dir(&target_dir_path) {
                    for ent_res in read_dir {
                        if let Ok(ent) = ent_res {
                            let name = ent.file_name().to_string_lossy().to_string();

                            if !is_all && name.to_string().starts_with('.') {
                                continue;
                            }

                            let file_type = get_classify_type(
                                &join_path(&target_dir_path, &name)
                            ).unwrap();

                            let mut info = FileInfo {
                                name: (&name).to_string(),
                                r#type: file_type,
                                full_path: join_path(&target_dir_path.to_string(), &name),
                                ..Default::default()
                            };
                            if _is_listing {
                                populate_listing_info(&mut info);
                            }
                            entries.push(info);
                        }
                    }
                }
            }
            Err(error) => {
                return Err(anyhow!(error));
            }
            _ => {}
        }

        results.push(Directory {
            name: display_name.to_string(),
            file_content: entries,
        });
    }

    Ok(results)
}

fn join_path(absolute_path: &str, subfolder: &str) -> String {
    let base_path = Path::new(absolute_path);
    let joined_path = base_path.join(subfolder);
    joined_path.to_string_lossy().to_string()
}

fn get_classify_type(path: &str) -> CrateResult<String> {
    use std::os::unix::fs::FileTypeExt;
    let metadata = std::fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();

    if metadata.is_dir() {
        Ok("directory".to_string())
    } else if file_type.is_symlink() {
        Ok("symlink".to_string())
    } else if file_type.is_char_device() {
        Ok("char device".to_string())
    } else if file_type.is_block_device() {
        Ok("block device".to_string())
    } else if file_type.is_fifo() {
        Ok("pipe".to_string())
    } else if file_type.is_socket() {
        Ok("socket".to_string())
    } else {
        Ok("file".to_string())
    }
}

pub fn display_ls_result(
    _is_all: bool,
    is_classify: bool,
    is_listing: bool,
    data: Vec<Directory>
) -> String {
    let mut result: String = String::new();

    for (idx, dir) in data.iter().enumerate() {
        let mut file_content = dir.file_content.clone();
        file_content.sort_by(|a, b| {
            // Special handling for . and .. entries
            match (&a.name[..], &b.name[..]) {
                (".", _) => std::cmp::Ordering::Less,
                (_, ".") => std::cmp::Ordering::Greater,
                ("..", _) if b.name != "." => std::cmp::Ordering::Less,
                (_, "..") if a.name != "." => std::cmp::Ordering::Greater,
                _ => {
                    // Then sort alphabetically (case-insensitive)
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            }
        });

        if data.len() > 1 {
            result.push_str(&format!("{}:", &dir.name));
            result.push('\n');
        }

        if is_listing {
            // Compute column widths
            let mut max_links = 0usize;
            let mut max_user = 0usize;
            let mut max_group = 0usize;
            let mut max_size = 0usize;
            let mut max_device_width = 0usize;

            for f in &file_content {
                max_links = max_links.max(f.links.to_string().len());
                max_user = max_user.max(f.user.len());
                max_group = max_group.max(f.group.len());

                // Calculate device width for block/char devices
                if f.r#type == "block_device" || f.r#type == "char_device" {
                    let (_, dev_id) = f.device_info;
                    let (major, minor) = extract_major_minor(dev_id);
                    let device_str = format!("{}, {}", major, minor);
                    max_device_width = max_device_width.max(device_str.len());
                } else {
                    max_size = max_size.max(f.size.to_string().len());
                }
            }

            // Ensure minimum width alignment with regular file sizes
            max_device_width = max_device_width.max(max_size);

            let total_kblocks: u64 = {
                #[cfg(unix)]
                {
                    let mut blocks_512: u64 = 0;
                    for f in &file_content {
                        if let Ok(md) = std::fs::symlink_metadata(&f.full_path) {
                            blocks_512 += md.blocks();
                        }
                    }
                    (blocks_512.saturating_mul(512) + 1023) / 1024
                }
            };
            result.push_str(&format!("total {}\n", total_kblocks));

            for file in &file_content {
                let perms = if let Some(p) = file.permissions.get(0) { p } else { "---------" };
                let type_char = file_type_char(&file.r#type);
                let mut name_segment = file.name.clone();
                if is_classify {
                    name_segment.push_str(add_classify_syntax(&file.r#type));
                }
                if file.r#type == "symlink" {
                    if let Some(target) = &file.symlink_target {
                        name_segment.push_str(" -> ");
                        name_segment.push_str(target);
                    }
                }

                match type_char {
                    v if v == 'b' || v == 'c' => {
                        let (_, dev_id) = file.device_info;
                        let (major, minor) = extract_major_minor(dev_id);
                        let device_str = format!("{}, {}", major, minor);

                        result.push_str(
                            &format!(
                                "{}{} {:>links$} {:<userw$} {:<groupw$} {:>devicew$} {} {}\n",
                                type_char,
                                perms,
                                file.links,
                                file.user,
                                file.group,
                                device_str,
                                file.modified_time,
                                name_segment,
                                links = max_links,
                                userw = max_user,
                                groupw = max_group,
                                devicew = max_device_width
                            )
                        );
                    }
                    _ => {
                        result.push_str(
                            &format!(
                                "{}{} {:>links$} {:<userw$} {:<groupw$} {:>sizew$} {} {}\n",
                                type_char,
                                perms,
                                file.links,
                                file.user,
                                file.group,
                                file.size,
                                file.modified_time,
                                name_segment,
                                links = max_links,
                                userw = max_user,
                                groupw = max_group,
                                sizew = max_size
                            )
                        );
                    }
                }
            }
        } else {
            for (idx, file) in file_content.iter().enumerate() {
                if idx != 0 {
                    result.push_str("  ");
                }
                result.push_str(&file.name);
                if is_classify {
                    result.push_str(add_classify_syntax(&file.r#type));
                }
                if idx == file_content.len() - 1 {
                    result.push_str("\n");
                }
            }
        }
        
        if idx<data.len()-1 {
            result.push('\n');
        }
    }

    result
}

fn add_classify_syntax<'a>(file_type: &'a str) -> &'a str {
    match file_type {
        "directory" => "/",
        "executable" => "*",
        "socket" => "=",
        "pipe" => "|",
        _ => "",
    }
}

fn extract_major_minor(dev_id: u64) -> (usize, usize) {
    (({ libc::major(dev_id) }) as usize, ({ libc::minor(dev_id) }) as usize)
}

fn populate_listing_info(info: &mut FileInfo) {
    // Default values
    let mut permissions_string = String::new();
    let mut permission_bits: usize = 0;
    let mut user_string = String::new();
    let mut group_string = String::new();
    let mut device_info: (u64, u64) = (0, 0);
    let mut symlink_target: Option<String> = None;
    let mut links: u64 = 0;
    let mut size: u64 = 0;
    let mut modified_time = String::new();

    // Prefer symlink metadata to avoid following links when determining type/target
    let meta_symlink = std::fs::symlink_metadata(&info.full_path);
    if let Ok(md) = meta_symlink {
        // Symlink target
        if md.file_type().is_symlink() {
            if let Ok(target) = std::fs::read_link(&info.full_path) {
                symlink_target = Some(target.to_string_lossy().to_string());
            }
        }

        {
            let mode = md.mode();
            permission_bits = (mode & 0o777) as usize;
            permissions_string = build_unix_permission_string(mode);

            user_string = resolve_unix_user(md.uid());
            group_string = resolve_unix_group(md.gid());

            device_info = (md.dev(), md.rdev());
            links = md.nlink() as u64;
            size = md.size();

            if let Ok(sys_time) = md.modified() {
                modified_time = format_time_unix(sys_time);
            }
        }
    }

    info.permission_bits = permission_bits;

    info.permissions = if permissions_string.is_empty() {
        Vec::new()
    } else {
        vec![permissions_string]
    };
    info.user = user_string;
    info.group = group_string;
    info.device_info = device_info;
    info.symlink_target = symlink_target;
    info.links = links;
    info.size = size;
    info.modified_time = modified_time;
}

#[cfg(unix)]
fn build_unix_permission_string(mode: u32) -> String {
    let mut perm_str = String::with_capacity(10);

    let owner = (mode & 0o700) >> 6;
    let group = (mode & 0o070) >> 3;
    let others = mode & 0o007;

    // Owner permissions
    perm_str.push(if (owner & 0o4) != 0 { 'r' } else { '-' });
    perm_str.push(if (owner & 0o2) != 0 { 'w' } else { '-' });
    if (mode & 0o4000) != 0 {
        perm_str.push(if (owner & 0o1) != 0 { 's' } else { 'S' });
    } else {
        perm_str.push(if (owner & 0o1) != 0 { 'x' } else { '-' });
    }

    // Group permissions
    perm_str.push(if (group & 0o4) != 0 { 'r' } else { '-' });
    perm_str.push(if (group & 0o2) != 0 { 'w' } else { '-' });
    if (mode & 0o2000) != 0 {
        perm_str.push(if (group & 0o1) != 0 { 's' } else { 'S' });
    } else {
        perm_str.push(if (group & 0o1) != 0 { 'x' } else { '-' });
    }

    // Others permissions
    perm_str.push(if (others & 0o4) != 0 { 'r' } else { '-' });
    perm_str.push(if (others & 0o2) != 0 { 'w' } else { '-' });
    if (mode & 0o1000) != 0 {
        perm_str.push(if (others & 0o1) != 0 { 't' } else { 'T' });
    } else {
        perm_str.push(if (others & 0o1) != 0 { 'x' } else { '-' });
    }

    perm_str
}

fn file_type_char(file_type: &str) -> char {
    match file_type {
        "directory" => 'd',
        "symlink" => 'l',
        "char device" => 'c',
        "block device" => 'b',
        "socket" => 's',
        "pipe" => 'p',
        _ => '-',
    }
}

#[cfg(unix)]
fn resolve_unix_user(uid: u32) -> String {
    // Use libc + passwd to resolve UID to name
    unsafe {
        let pwd = libc::getpwuid(uid);
        if pwd.is_null() {
            return uid.to_string();
        }
        let name_ptr = (*pwd).pw_name;
        if name_ptr.is_null() {
            return uid.to_string();
        }
        let c_str = std::ffi::CStr::from_ptr(name_ptr);
        c_str.to_string_lossy().into_owned()
    }
}

#[cfg(unix)]
fn resolve_unix_group(gid: u32) -> String {
    unsafe {
        let grp = libc::getgrgid(gid);
        if grp.is_null() {
            return gid.to_string();
        }
        let name_ptr = (*grp).gr_name;
        if name_ptr.is_null() {
            return gid.to_string();
        }
        let c_str = std::ffi::CStr::from_ptr(name_ptr);
        c_str.to_string_lossy().into_owned()
    }
}

#[cfg(unix)]
fn format_time_unix(time: std::time::SystemTime) -> String {
    use chrono::{ DateTime, Local };
    use chrono_tz::Tz;

    let name = iana_time_zone::get_timezone().unwrap_or("UTC".to_string());
    let tz = name.parse::<chrono_tz::Tz>().unwrap_or(Tz::UTC);
    let last_mod_time = time;
    let datetime: DateTime<Local> = last_mod_time.into();
    let datetime = datetime.with_timezone(&tz);

    let mut formatted_time = datetime.format("%b %e %H:%M").to_string();
    let current_year = Local::now().year();
    let its_year = datetime.year();
    if current_year != its_year {
        formatted_time = datetime.format("%b %e  %Y").to_string();
    }
    formatted_time
}

pub fn pwd() -> String {
    let cur_dir = std::env::current_dir().unwrap();
    cur_dir.display().to_string()
}
