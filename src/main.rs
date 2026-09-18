mod config;
mod digest;
mod grammar;
mod installed_detector;
mod network;
mod options;
mod schema;
mod yaml;

use anyhow::{Context, Result, bail};
use colored::Colorize;
use inquire::{Confirm, Select};
use std::{cmp::Ordering, fmt::Display, path::Path, process::ExitCode};
use strum::{Display, IntoEnumIterator};
use tokio::join;

use crate::config::Config;
use crate::grammar::LatestGrammar;
use crate::installed_detector::{InstalledGrammar, InstalledSchema};
use crate::network::Network;
use crate::options::{AuxCode, AuxMode, Pinyin, Schema};
use crate::schema::LatestSchema;

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
                all_schema.iter().map(|i| i.schema).collect(),
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

fn prompt_confirm() -> Result<bool> {
    Ok(Confirm::new("是否继续？").with_default(true).prompt()?)
}

fn prompt_finish() {
    println!("{}", "完成，请重新部署。".bright_green());
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
    let (installed_schema, installed_grammar) = installed?;
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

    match action {
        Action::Install => {
            let latest_schema = latest.schema.context("无法获取最新输入方案")?;

            let options = prompt_options(
                Required {
                    schema: true,
                    config: true,
                    with_grammar: true,
                },
                &Options::default(),
                true,
            )?;
            let schema = options.schema.unwrap();
            let config = options.config.unwrap();
            let with_grammar = options.with_grammar.unwrap();

            if with_grammar && latest.grammar.is_none() {
                bail!("无法获取最新语法模型");
            }

            schema::info_install(schema);
            config::info_apply(config);
            if with_grammar {
                grammar::info_install();
            }

            schema::warn_install(&root)?;
            config::warn_apply(&root, schema);
            if with_grammar {
                grammar::warn_install(&root);
            }

            if !prompt_confirm()? {
                return Ok(());
            }

            schema::update(&downloader, &root, schema, latest_schema)
                .await
                .context("更新输入方案失败")?;
            config::apply(&root, schema, config).context("应用方案配置失败")?;
            if with_grammar {
                grammar::update(&downloader, &root, &latest.grammar.unwrap()).await?;
            }

            prompt_finish();
        }

        Action::UpdateAll => {
            if !has_update.schema && !has_update.grammar {
                println!(
                    "{}",
                    "输入方案和语法模型均已是最新，无需更新。".bright_green()
                );
                return Ok(());
            }

            let options = prompt_options(
                Required {
                    schema: true,
                    ..Default::default()
                },
                &Options {
                    schema: installed_schema.map(|installed| installed.schema),
                    ..Default::default()
                },
                false,
            )?;
            let schema = options.schema.unwrap();

            if has_update.schema {
                schema::info_install(schema);
            }
            if has_update.grammar {
                grammar::info_install();
            }

            if has_update.schema {
                schema::warn_install(&root)?;
            }
            if has_update.grammar {
                grammar::warn_install(&root);
            }

            if !prompt_confirm()? {
                return Ok(());
            }

            if has_update.schema {
                schema::update(&downloader, &root, schema, latest.schema.unwrap())
                    .await
                    .context("更新输入方案失败")?;
            }
            if has_update.grammar {
                grammar::update(&downloader, &root, &latest.grammar.unwrap())
                    .await
                    .context("更新语法模型失败")?;
            }

            prompt_finish();
        }

        Action::UpdateSchema | Action::ForceUpdateSchema => {
            let latest_schema = latest.schema.context("无法获取最新输入方案")?;

            let options = prompt_options(
                Required {
                    schema: true,
                    ..Default::default()
                },
                &Options {
                    schema: installed_schema.map(|installed| installed.schema),
                    ..Default::default()
                },
                false,
            )?;
            let schema = options.schema.unwrap();

            schema::info_install(schema);

            schema::warn_install(&root)?;

            if !prompt_confirm()? {
                return Ok(());
            }

            schema::update(&downloader, &root, schema, latest_schema)
                .await
                .context("更新输入方案失败")?;

            prompt_finish();
        }

        Action::SwitchSchema => {
            let options = prompt_options(
                Required {
                    schema: true,
                    config: true,
                    ..Default::default()
                },
                &Options {
                    schema: installed_schema.as_ref().map(|installed| installed.schema),
                    config: installed_schema.as_ref().map(|installed| installed.config),
                    ..Default::default()
                },
                true,
            )?;
            let schema = options.schema.unwrap();
            let config = options.config.unwrap();

            let install_schema = installed_schema
                .as_ref()
                .is_none_or(|installed| installed.schema != schema);
            let apply_config = installed_schema
                .as_ref()
                .is_none_or(|installed| installed.config != config);

            if !install_schema && !apply_config {
                println!("{}", "输入方案和方案配置无变动。".bright_green());
                return Ok(());
            }

            if install_schema && latest.schema.is_none() {
                bail!("无法获取最新输入方案");
            }

            if install_schema {
                schema::info_install(schema);
            }
            if apply_config {
                config::info_apply(config);
            }

            if install_schema {
                schema::warn_install(&root)?;
            }
            if apply_config {
                config::warn_apply(&root, schema);
            }

            if !prompt_confirm()? {
                return Ok(());
            }

            if install_schema {
                schema::update(&downloader, &root, schema, latest.schema.unwrap())
                    .await
                    .context("更新输入方案失败")?;
                schema::cleanup(&root, installed_schema.unwrap().schema, schema);
            }
            if apply_config {
                config::apply(&root, schema, config).context("应用方案配置失败")?;
            }

            prompt_finish();
        }

        Action::InstallGrammar | Action::UpdateGrammar | Action::ForceUpdateGrammar => {
            let latest_grammar = latest.grammar.context("无法获取最新语法模型")?;

            grammar::info_install();

            grammar::warn_install(&root);

            if !prompt_confirm()? {
                return Ok(());
            }

            grammar::update(&downloader, &root, &latest_grammar)
                .await
                .context("更新语法模型失败")?;

            prompt_finish();
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
