mod config;
mod digest;
mod downloader;
mod grammar;
mod installed_detector;
mod options;
mod scheme;
mod yaml;

use anyhow::{Context, Result};
use colored::Colorize;
use downloader::Downloader;
use inquire::{Confirm, Select};
use installed_detector::{InstalledGrammar, InstalledSchema};
use octocrab::models::repos::{Asset, Release};
use options::{AuxCode, AuxMode, Pinyin, Scheme};
use std::{cmp::Ordering, fmt::Display, path::Path, process::ExitCode};
use strum::{Display, IntoEnumIterator};
use tokio::join;

pub(crate) fn print_err(e: impl Display) {
    eprintln!("{}", format!("错误：{e:#}").red());
}

fn resolve_installed(root: &Path) -> Result<(Option<InstalledSchema>, Option<InstalledGrammar>)> {
    let all_schema = InstalledSchema::detect(root);
    let grammar = InstalledGrammar::detect(root);

    let current_schema = match all_schema.len().cmp(&1) {
        Ordering::Equal => Some(all_schema[0].clone()),
        Ordering::Greater => {
            let i = Select::new(
                "检测到多个主方案，请指定",
                all_schema.iter().map(|i| i.scheme).collect(),
            )
            .raw_prompt()?
            .index;
            Some(all_schema[i].clone())
        }
        Ordering::Less => None,
    };

    Ok((current_schema, grammar))
}

struct LatestInfo {
    schema: Option<Release>,
    grammar: Option<Asset>,
}

async fn get_latest() -> Result<LatestInfo> {
    let schema = scheme::get_latest(false); // TODO: prerelease option
    let grammar = grammar::get_latest();
    let (schema, grammar) = join!(schema, grammar);

    let schema = schema
        .context("获取最新输入方案失败")
        .inspect_err(|e| print_err(e))
        .ok();
    let grammar = grammar
        .context("获取最新语法模型失败")
        .inspect_err(|e| print_err(e))
        .ok();
    Ok(LatestInfo { schema, grammar })
}

fn check_update(schema: &InstalledSchema, grammar: Option<&InstalledGrammar>, latest: &LatestInfo) {
    if let Some(latest) = &latest.schema {
        match scheme::check_update(schema, latest).context("检查输入方案更新失败") {
            Ok(has_update) => println!(
                "输入方案： v{} -> {}",
                schema.version,
                if has_update {
                    latest.tag_name.yellow()
                } else {
                    "已是最新".green()
                }
            ),
            Err(e) => print_err(e),
        }
    }

    if let Some(installed) = grammar {
        if let Some(latest) = &latest.grammar {
            match grammar::check_update(&installed.path, latest).context("检查语法模型更新失败")
            {
                Ok(has_update) => println!(
                    "语法模型： {}",
                    if has_update {
                        format!("有更新 {}", latest.updated_at.with_timezone(&chrono::Local))
                    } else {
                        String::from("已是最新")
                    }
                ),
                Err(e) => print_err(e),
            }
        }
    } else {
        println!("语法模型： 未安装");
    }
}

fn select<T>(prompt: &str, options: Vec<T>, default: Option<&T>) -> Result<T>
where
    T: PartialEq + Display,
{
    let default_index = default
        .and_then(|default| options.iter().position(|option| option == default))
        .unwrap_or(0);
    Ok(Select::new(prompt, options)
        .with_starting_cursor(default_index)
        .prompt()?)
}

fn prompt_install(root: &Path, install_scheme: bool, install_grammar: bool) -> Result<bool> {
    if install_scheme {
        scheme::prompt_install(root)?;
    }
    if install_grammar {
        grammar::prompt_install(root);
    }
    Ok(Confirm::new("是否继续？").with_default(true).prompt()?)
}

#[derive(Clone, Display, PartialEq)]
enum Action {
    Install,
    #[strum(to_string = "更新全部")]
    UpdateAll,
    #[strum(to_string = "更新输入方案")]
    UpdateScheme,
    #[strum(to_string = "安装语法模型")]
    InstallGrammar,
    #[strum(to_string = "更新语法模型")]
    UpdateGrammar,
    #[strum(to_string = "切换方案")]
    SwitchScheme,
    #[strum(to_string = "退出")]
    Exit,
}

async fn run() -> Result<()> {
    let root = std::env::current_dir()?;
    let downloader = Downloader::new()?;

    let latest = get_latest();
    let installed = resolve_installed(&root);
    let (installed_schema, installed_grammar) = installed?;
    let latest = latest.await?;

    let action = match &installed_schema {
        None => Action::Install,
        Some(installed_schema) => {
            check_update(installed_schema, installed_grammar.as_ref(), &latest);

            Select::new("选择操作", {
                let mut actions = Vec::new();
                if installed_grammar.is_some() {
                    actions.push(Action::UpdateAll);
                }
                actions.push(Action::UpdateScheme);
                actions.push(match &installed_grammar {
                    Some(_) => Action::UpdateGrammar,
                    None => Action::InstallGrammar,
                });
                actions.push(Action::SwitchScheme);
                actions.push(Action::Exit);
                actions
            })
        }
        .prompt()?,
    };

    match action {
        Action::Install | Action::UpdateAll | Action::UpdateScheme | Action::SwitchScheme => {
            let latest_schema = latest.schema.context("未获取到最新输入方案")?;

            #[allow(clippy::items_after_statements)]
            fn select_scheme(default: Option<&Scheme>) -> Result<Scheme> {
                select("选择输入方案", Scheme::iter().collect(), default)
            }
            let scheme = if action == Action::SwitchScheme {
                select_scheme(
                    installed_schema
                        .as_ref()
                        .map(|installed| installed.scheme)
                        .as_ref(),
                )?
            } else {
                match &installed_schema.as_ref().map(|installed| installed.scheme) {
                    Some(scheme) => *scheme,
                    None => select_scheme(None)?,
                }
            };

            let pinyin = if matches!(action, Action::Install | Action::SwitchScheme) {
                let default = installed_schema
                    .as_ref()
                    .and_then(|installed| installed.pinyin);
                Some(select(
                    "选择拼音方案",
                    Pinyin::iter().collect(),
                    default.as_ref(),
                )?)
            } else {
                None
            };

            let scheme = if scheme == Scheme::Pro(None) {
                Scheme::Pro(select("选择辅助码包体", AuxCode::iter().collect(), None)?.into())
            } else {
                scheme
            };

            let aux_mode = if matches!(scheme, Scheme::Pro(_))
                && matches!(action, Action::Install | Action::SwitchScheme)
            {
                let default = installed_schema
                    .as_ref()
                    .and_then(|installed| installed.aux_mode);
                Some(select(
                    "选择辅助码引导模式",
                    AuxMode::iter().collect(),
                    default.as_ref(),
                )?)
            } else {
                None
            };

            let with_grammar = match action {
                Action::Install => Confirm::new("是否安装语法模型？")
                    .with_default(true)
                    .prompt()?,
                Action::UpdateAll => true,
                _ => false,
            };

            println!("{} 将安装输入方案：", "==>".bright_green());
            println!("  方案：{}", scheme.to_string().bright_cyan());
            if let Scheme::Pro(aux) = scheme {
                println!("  辅助码方案：{}", aux.unwrap().to_string().bright_cyan());
            }

            if matches!(action, Action::Install | Action::SwitchScheme) {
                println!("{} 将应用方案配置：", "==>".bright_green());
                if let Some(pinyin) = pinyin {
                    println!("  拼音方案：{}", pinyin.to_string().bright_cyan());
                }
                if let Some(aux_mode) = aux_mode {
                    println!("  辅助码引导模式：{}", aux_mode.to_string().bright_cyan());
                }
            }

            if !prompt_install(&root, true, with_grammar)? {
                return Ok(());
            }

            scheme::update(&downloader, &root, scheme, latest_schema).await?;

            if matches!(action, Action::Install | Action::SwitchScheme) {
                config::apply(&root, scheme, pinyin, aux_mode).context("应用方案配置失败")?;
            }

            if action == Action::SwitchScheme {
                scheme::cleanup(&root, installed_schema.unwrap().scheme, scheme);
            }

            if with_grammar {
                let latest = latest.grammar.context("未获取到最新语法模型")?;
                grammar::update(&downloader, &root, &latest).await?;
            }
        }

        Action::InstallGrammar | Action::UpdateGrammar => {
            let latest = latest.grammar.context("未获取到最新语法模型")?;

            if !prompt_install(&root, false, true)? {
                return Ok(());
            }

            return grammar::update(&downloader, &root, &latest).await;
        }

        Action::Exit => return Ok(()),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", format!("错误：{e:?}").red());
            ExitCode::FAILURE
        }
    }
}
