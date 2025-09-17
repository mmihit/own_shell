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

#[derive(Debug,PartialEq)]
pub struct Ls {
    pub flag: String,
    pub dirs: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Rm {
    pub is_dir: bool,
    pub dirs: Vec<String>,
}

impl Ls {
    fn from(flag: String, dirs: Vec<String>) -> Self {
        Self { flag, dirs }
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
        let input_slice  = split_preserve_quotes_simple(input);
        match input_slice[0].to_lowercase().as_str() {
            "exit" => Ok(Self::Exit),

            "pwd" => if input_slice.len()<2 {return Ok(Self::Pwd)} else {Err(anyhow!("too many arguments/options"))},

            "cd" => if input_slice.len() > 2 {
                return Err(anyhow!("cd requires one arguments"));
            } else {
                return Ok(Self::Cd(input_slice[1..].join(" ")));
            }

            "ls" => if input_slice.len() > 1 {
                match input_slice[1].as_str() {
                    "-l" | "-a" | "-F" =>
                        Ok(
                            Self::Ls(
                                Ls::from(
                                    input_slice[1].to_string(),
                                    input_slice[2..]
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect()
                                )
                            )
                        ),
                    v if v.chars().nth(0) == Some('-') => {
                        return Err(
                            anyhow!("invalid option gf <{v}>, expected one of this args: -l, -a, -F tezzzz")
                        );
                    }
                    _ =>
                        Ok(
                            Self::Ls(
                                Ls::from(
                                    String::new(),
                                    input_slice[1..]
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect()
                                )
                            )
                        ),
                }
            } else {
                return Ok(Self::Ls(Ls::from(String::new(), vec![])));
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
                match input_slice[1].as_str()   {
                    "-r"=> if input_slice.len() > 2 {
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
                    )
                    } else {
                        return Err(anyhow!("missing a path"))
                    },
                    v if v.chars().nth(0) == Some('-') => return Err(anyhow!("invalid option <{v}>, expected options: -r")),
                    _=> return Ok(Self::Rm(Rm::from(false, input_slice[1..].iter().map(|s| s.to_string()).collect())))
                }

            }

            "mv" => if input_slice.len() < 3 {
                return Err(anyhow!("mv requires at least two arguments: ccxsource(s) and destination"));
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

fn split_preserve_quotes_simple(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_token = String::new();
    let mut inside_single_quotes = false;
    let mut inside_double_quotes = false;
    
    for ch in input.chars() {
        match ch {
            '\'' if !inside_double_quotes => {
                inside_single_quotes = !inside_single_quotes;
                current_token.push(ch);
            },
            '"' if !inside_single_quotes => {
                inside_double_quotes = !inside_double_quotes;
                current_token.push(ch);
            },
            ' ' if !inside_single_quotes && !inside_double_quotes => {
                if !current_token.is_empty() {
                    result.push(current_token.clone());
                    current_token.clear();
                }
            },
            _ => {
                current_token.push(ch);
            }
        }
    }
    
    if !current_token.is_empty() {
        result.push(current_token);
    }
    
    result
}