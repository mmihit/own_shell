use anyhow::{ anyhow, Ok };

#[derive(Debug, PartialEq)]
pub enum Command {
    Echo(String),
    Cd(String),
    Ls(Ls),
    Pwd,
    Cat(Vec<String>),
    Cp(Vec<String>),
    Rm(Rm),
    Mv(Vec<String>),
    Mkdir(Vec<String>),
    Exit,
}

#[derive(Debug, PartialEq)]
pub struct Ls {
    pub is_all: bool,
    pub is_classify: bool,
    pub is_listing: bool,
    pub dirs: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Rm {
    pub is_dir: bool,
    pub dirs: Vec<String>,
}

impl Ls {
    // fn from(is_all: bool, is_classify: bool, is_listing: bool, dirs: Vec<String>) -> Self {
    //     Self { is_all, is_classify, is_listing, dirs }
    // }

    fn new() -> Self {
        Self {
            is_all: false,
            is_classify: false,
            is_listing: false,
            dirs: vec![],
        }
    }
}

impl Rm {
    fn from(is_dir: bool, dirs: Vec<String>) -> Self {
        Self { is_dir, dirs }
    }
}

impl TryFrom<&str> for Command {
    type Error = anyhow::Error;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let input_slice: Vec<&str> = input.split(" ").collect();
        match input_slice[0].to_lowercase().as_str() {
            "exit" => Ok(Self::Exit),

            "pwd" => if input_slice.len() < 2 {
                return Ok(Self::Pwd);
            } else {
                Err(anyhow!("too many arguments/options"))
            }

            "cd" => if input_slice.len() > 2 {
                return Err(anyhow!("cd requires one arguments"));
            } else {
                return Ok(Self::Cd(input_slice[1..].join(" ")));
            }

            "ls" => {
                let mut result = Ls::new();
                if input_slice.len() > 1 {
                    for v in &input_slice[1..] {
                        if v.starts_with("-") {
                            for ch in v.chars().skip(1) {
                                match ch {
                                    'a' => {
                                        result.is_all = true;
                                    }
                                    'F' => {
                                        result.is_classify = true;
                                    }
                                    'l' => {
                                        result.is_listing = true;
                                    }
                                    _ => {
                                        return Err(anyhow!("invalid option -{ch}"));
                                    }
                                }
                            }
                        } else {
                            result.dirs.push(v.to_string());
                        }
                    }
                }
                if result.dirs.len() == 0 {
                    result.dirs.push(String::from("."))
                }
                return Ok(Self::Ls(result))
            }

            "echo" => if input_slice.len() < 2 {
                return Err(anyhow!("echo requires an argument"));
            } else {
                return Ok(Self::Echo(input_slice[1..].join(" ")));
            }

            "cat" => if input_slice.len() < 2 {
                return Err(anyhow!("cat requires an argument"));
            } else {
                return Ok(
                    Self::Cat(
                        input_slice[1..]
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    )
                );
            }

            "cp" => if input_slice.len() != 3 {
                return Err(anyhow!("cp requires two arguments: source & target"));
            } else {
                return Ok(
                    Self::Cp(
                        input_slice[1..]
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    )
                );
            }

            "rm" => if input_slice.len() < 2 {
                return Err(anyhow!("rm requires at least one argument"));
            } else {
                match input_slice[1] {
                    "-r" => if input_slice.len() > 2 {
                        return Ok(
                            Self::Rm(
                                Rm::from(
                                    true,
                                    input_slice[2..]
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect()
                                )
                            )
                        );
                    } else {
                        return Err(anyhow!("missing a path"));
                    }
                    v if v.chars().nth(0) == Some('-') => {
                        return Err(anyhow!("invalid option <{v}>, expected options: -r"));
                    }
                    _ => {
                        return Ok(
                            Self::Rm(
                                Rm::from(
                                    false,
                                    input_slice[1..]
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect()
                                )
                            )
                        );
                    }
                }
            }

            "mv" => if input_slice.len() < 3 {
                return Err(
                    anyhow!("mv requires at least two arguments: ccxsource(s) and destination")
                );
            } else {
                return Ok(
                    Self::Mv(
                        input_slice[1..]
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    )
                );
            }

            "mkdir" => if input_slice.len() < 2 {
                return Err(anyhow!("mkdir requires at least one argument"));
            } else {
                return Ok(
                    Self::Mkdir(
                        input_slice[1..]
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    )
                );
            }
            y => Err(anyhow!(format!("command <{}> not found", y))),
        }
    }
}

// fn split_preserve_quotes_simple(input: &str) -> Vec<String> {
//     let mut result = Vec::new();
//     let mut current_token = String::new();
//     let mut inside_single_quotes = false;
//     let mut inside_double_quotes = false;

//     for ch in input.chars() {
//         match ch {
//             '\'' if !inside_double_quotes => {
//                 inside_single_quotes = !inside_single_quotes;
//                 current_token.push(ch);
//             },
//             '"' if !inside_single_quotes => {
//                 inside_double_quotes = !inside_double_quotes;
//                 current_token.push(ch);
//             },
//             ' ' if !inside_single_quotes && !inside_double_quotes => {
//                 if !current_token.is_empty() {
//                     result.push(current_token.clone());
//                     current_token.clear();
//                 }
//             },
//             _ => {
//                 current_token.push(ch);
//             }
//         }
//     }

//     if !current_token.is_empty() {
//         result.push(current_token);
//     }

//     result
// }
