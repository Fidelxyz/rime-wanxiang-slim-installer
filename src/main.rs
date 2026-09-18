mod digest;
mod modules;
mod network;
mod options;
mod workflow;
mod yaml;

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, Select};
use std::{cmp::Ordering, fmt::Display, path::Path, process::ExitCode};
use strum::{Display, IntoEnumIterator};
use tokio::join;

use crate::modules::config::Config;
use crate::modules::grammar::{InstalledGrammar, LatestGrammar};
use crate::modules::schema::{InstalledSchema, LatestSchema};
use crate::modules::{config, grammar, schema};
use crate::network::Network;
use crate::options::{AuxCode, AuxMode, Pinyin, Schema};
use crate::workflow::Workflow;

pub(crate) fn print_err(e: impl Display) {
    eprintln!("{}", format!("错误：{e:#}").red());
}

fn resolve_installed(
    root: &Path,
) -> Result<(
    Option<InstalledSchema>,
    Option<InstalledGrammar>,
    Option<Config>,
)> {
    let all_schema = schema::detect(root);
    let grammar = grammar::detect(root);

    let current_schema = match all_schema.len().cmp(&1) {
        Ordering::Equal => Some(all_schema[0].clone()),
        Ordering::Greater => {
            let i = Select::new(
                "检测到多个主方案，请指定",
                all_schema.iter().map(|i| i.schema).collect(),
            )
            .raw_prompt()?
            .index;
            Some(all_schema[i].clone())
        }
        Ordering::Less => None,
    };

    let config = current_schema
        .as_ref()
        .map(|installed| config::detect(root, installed.schema))
        .transpose()?;

    Ok((current_schema, grammar, config))
}

struct LatestInfo {
    schema: Option<LatestSchema>,
    grammar: Option<LatestGrammar>,
}

async fn get_latest() -> Result<LatestInfo> {
    let schema = schema::get_latest(false); // TODO: prerelease option
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

#[derive(Default)]
struct HasUpdate {
    schema: bool,
    grammar: bool,
}

async fn check_update(
    downloader: &Network,
    schema: &InstalledSchema,
    grammar: Option<&InstalledGrammar>,
    latest: &LatestInfo,
) -> HasUpdate {
    let schema_has_update = latest.schema.as_ref().is_some_and(|latest| {
        schema::check_update(schema, latest)
            .context("检查输入方案更新失败")
            .inspect(|&has_update| {
                println!(
                    "输入方案： v{} -> {}",
                    schema.version,
                    if has_update {
                        latest.release.tag_name.yellow()
                    } else {
                        "已是最新".green()
                    }
                );
            })
            .inspect_err(|e| print_err(e))
            .unwrap_or(false)
    });

    let grammar_has_update = match (grammar, latest.grammar.as_ref()) {
        (None, _) => {
            println!("语法模型： 未安装");
            false
        }
        (Some(installed), Some(latest)) => {
            grammar::check_update(downloader, &installed.path, latest)
                .await
                .context("检查语法模型更新失败")
                .inspect(|&has_update| {
                    println!(
                        "语法模型： {}",
                        if has_update {
                            format!(
                                "有更新 {}",
                                latest
                                    .asset
                                    .updated_at
                                    .with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d %H:%M:%S %Z")
                            )
                            .yellow()
                        } else {
                            "已是最新".green()
                        }
                    );
                })
                .inspect_err(|e| print_err(e))
                .unwrap_or(false)
        }
        _ => false,
    };

    HasUpdate {
        schema: schema_has_update,
        grammar: grammar_has_update,
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

#[derive(Default)]
struct Options {
    schema: Option<Schema>,
    config: Option<Config>,
    with_grammar: Option<bool>,
}

#[derive(Clone, Copy, Default)]
struct Required {
    schema: bool,
    config: bool,
    with_grammar: bool,
}

fn prompt_options(required: Required, default: &Options, always_ask: bool) -> Result<Options> {
    let schema = if required.schema {
        match default.schema.filter(|_| !always_ask) {
            Some(schema) => schema,
            None => select(
                "选择输入方案",
                Schema::iter().collect(),
                default.schema.as_ref(),
            )?,
        }
        .into()
    } else {
        None
    };

    let pinyin = if required.config {
        let default = default.config.as_ref().and_then(|config| config.pinyin);
        select("选择拼音方案", Pinyin::iter().collect(), default.as_ref())?.into()
    } else {
        None
    };

    let schema = if schema == Some(Schema::Pro(None)) {
        Schema::Pro(select("选择辅助码包体", AuxCode::iter().collect(), None)?.into()).into()
    } else {
        schema
    };

    let aux_mode = if matches!(schema, Some(Schema::Pro(_))) {
        match default
            .config
            .as_ref()
            .and_then(|config| config.aux_mode)
            .filter(|_| !always_ask)
        {
            Some(aux_mode) => aux_mode,
            None => select(
                "选择辅助码引导模式",
                AuxMode::iter().collect(),
                default
                    .config
                    .as_ref()
                    .and_then(|config| config.aux_mode)
                    .as_ref(),
            )?,
        }
        .into()
    } else {
        None
    };

    let with_grammar = if required.with_grammar {
        Confirm::new("是否安装语法模型？")
            .with_default(true)
            .prompt()?
            .into()
    } else {
        None
    };

    Ok(Options {
        schema,
        config: if required.config {
            Some(Config { pinyin, aux_mode })
        } else {
            None
        },
        with_grammar,
    })
}

macro_rules! prompt_options {
    ($default:expr, $always_ask:expr; $($field:ident),+ $(,)?) => {{
        #[allow(clippy::needless_update)]
        let required = Required {
            $($field: true,)+
            ..Default::default()
        };
        prompt_options(required, $default, $always_ask)
            .map(|options| ($(options.$field.unwrap(),)+))
    }};
}

#[derive(Clone, Display, PartialEq)]
enum Action {
    Install,
    #[strum(to_string = "更新全部")]
    UpdateAll,
    #[strum(to_string = "更新输入方案")]
    UpdateSchema,
    #[strum(to_string = "更新输入方案（强制更新）")]
    ForceUpdateSchema,
    #[strum(to_string = "安装语法模型")]
    InstallGrammar,
    #[strum(to_string = "更新语法模型")]
    UpdateGrammar,
    #[strum(to_string = "更新语法模型（强制更新）")]
    ForceUpdateGrammar,
    #[strum(to_string = "切换方案")]
    SwitchSchema,
    #[strum(to_string = "退出")]
    Exit,
}

async fn run() -> Result<()> {
    let root = std::env::current_dir()?;
    let downloader = Network::new()?;

    let latest = get_latest();
    let installed = resolve_installed(&root);
    let (installed_schema, installed_grammar, installed_config) = installed?;
    let latest = latest.await?;
    let mut has_update = HasUpdate::default();

    let action = match &installed_schema {
        None => Action::Install,
        Some(installed_schema) => {
            has_update = check_update(
                &downloader,
                installed_schema,
                installed_grammar.as_ref(),
                &latest,
            )
            .await;

            Select::new("选择操作", {
                vec![
                    Action::UpdateAll,
                    if has_update.schema {
                        Action::UpdateSchema
                    } else {
                        Action::ForceUpdateSchema
                    },
                    match (installed_grammar, has_update.grammar) {
                        (None, _) => Action::InstallGrammar,
                        (Some(_), true) => Action::UpdateGrammar,
                        (Some(_), false) => Action::ForceUpdateGrammar,
                    },
                    Action::SwitchSchema,
                    Action::Exit,
                ]
            })
            .prompt()?
        }
    };

    let default_options = Options {
        schema: installed_schema.as_ref().map(|installed| installed.schema),
        config: installed_config,
        ..Default::default()
    };
    let mut workflow = Workflow::default();
    match action {
        Action::Install => {
            let latest_schema = latest.schema.context("无法获取最新输入方案")?;

            let (schema, config, with_grammar) =
                prompt_options!(&default_options, true; schema, config, with_grammar)?;

            workflow.register(schema::Install {
                schema,
                latest: latest_schema,
                previous: None,
            });
            workflow.register(config::Apply { schema, config });
            if with_grammar {
                workflow.register(grammar::Install {
                    latest: latest.grammar.context("无法获取最新语法模型")?,
                });
            }
        }

        Action::UpdateAll => {
            if !has_update.schema && !has_update.grammar {
                println!(
                    "{}",
                    "输入方案和语法模型均已是最新，无需更新。".bright_green()
                );
                return Ok(());
            }

            if has_update.schema {
                let (schema,) = prompt_options!(&default_options, false; schema)?;
                workflow.register(schema::Install {
                    schema,
                    latest: latest.schema.context("无法获取最新输入方案")?,
                    previous: None,
                });
            }
            if has_update.grammar {
                workflow.register(grammar::Install {
                    latest: latest.grammar.context("无法获取最新语法模型")?,
                });
            }
        }

        Action::UpdateSchema | Action::ForceUpdateSchema => {
            let latest_schema = latest.schema.context("无法获取最新输入方案")?;
            let (schema,) = prompt_options!(&default_options, false; schema)?;

            workflow.register(schema::Install {
                schema,
                latest: latest_schema,
                previous: None,
            });
        }

        Action::SwitchSchema => {
            let (schema, config) = prompt_options!(&default_options, true; schema, config)?;

            let install_schema = installed_schema
                .as_ref()
                .is_none_or(|installed| installed.schema != schema);
            let apply_config = installed_config != Some(config);
            if !install_schema && !apply_config {
                println!("{}", "输入方案和方案配置无变动。".bright_green());
                return Ok(());
            }

            if install_schema {
                workflow.register(schema::Install {
                    schema,
                    latest: latest.schema.context("无法获取最新输入方案")?,
                    previous: installed_schema.as_ref().map(|installed| installed.schema),
                });
            }
            if apply_config {
                workflow.register(config::Apply { schema, config });
            }
        }

        Action::InstallGrammar | Action::UpdateGrammar | Action::ForceUpdateGrammar => {
            workflow.register(grammar::Install {
                latest: latest.grammar.context("无法获取最新语法模型")?,
            });
        }

        Action::Exit => return Ok(()),
    }

    workflow.execute(&downloader, &root).await
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
