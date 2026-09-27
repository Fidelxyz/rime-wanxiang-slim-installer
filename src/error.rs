use colored::Colorize;
use std::{error::Error, fmt};

#[derive(Debug)]
pub struct UserCancelled;

impl fmt::Display for UserCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("已取消安装")
    }
}

impl Error for UserCancelled {}

pub fn print_err(e: &impl fmt::Display) {
    eprintln!("{}", format!("错误：{e:#}").red());
}
