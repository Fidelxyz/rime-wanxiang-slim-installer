use anyhow::{Context, Result};
use colored::Colorize;
use octocrab::models::repos::Asset;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::network::Network;
use crate::workflow::{ApplyFuture, Module};

pub struct InstalledGrammar {
    pub path: PathBuf,
}

pub fn detect(root: &Path) -> Option<InstalledGrammar> {
    let path = root.join(GRAMMAR_NAME);
    if !path.exists() {
        return None;
    }
    Some(InstalledGrammar { path })
}

pub struct Install {
    pub latest: LatestGrammar,
}

impl Module for Install {
    fn info(&self) {
        info_install();
    }

    fn warn(&self, root: &Path) -> Result<()> {
        warn_install(root);
        Ok(())
    }

    fn apply<'a>(self: Box<Self>, downloader: &'a Network, root: &'a Path) -> ApplyFuture<'a> {
        Box::pin(async move {
            update(downloader, root, &self.latest)
                .await
                .context("更新语法模型失败")
        })
    }
}

pub const GRAMMAR_NAME: &str = "wanxiang-lts-zh-hans.gram";

#[derive(Clone, PartialEq)]
pub struct LatestGrammar {
    pub asset: Asset,
}

pub async fn get_latest() -> Result<LatestGrammar> {
    let asset = octocrab::instance()
        .repos("amzxyz", "RIME-LMDG")
        .releases()
        .get_by_tag("LTS")
        .await?
        .assets
        .into_iter()
        .find(|asset| asset.name == GRAMMAR_NAME)
        .context("Release 中未找到语法模型 Asset")?;
    Ok(LatestGrammar { asset })
}

pub async fn check_update(
    downloader: &Network,
    path: &Path,
    latest: &LatestGrammar,
) -> Result<bool> {
    downloader.check_update(path, &latest.asset).await
}

fn info_install() {
    println!("{} 将安装语法模型：", "==>".bright_green());
    println!("  语法模型：{}", GRAMMAR_NAME.bright_cyan());
}

fn warn_install(root: &Path) {
    println!(
        "{}",
        format!("将安装语法模型至：{}", root.join(GRAMMAR_NAME).display()).bright_yellow()
    );
    if root.join(GRAMMAR_NAME).exists() {
        println!("{}", "原有文件将被覆盖。".bright_yellow());
    }
}

async fn update(downloader: &Network, root: &Path, latest: &LatestGrammar) -> Result<()> {
    let downloaded = downloader
        .download(&latest.asset)
        .await
        .context("语法模型下载失败")?;
    install(downloaded, &root.join(GRAMMAR_NAME)).context("语法模型安装失败")?;
    Ok(())
}

fn install(file: NamedTempFile, path: &Path) -> Result<()> {
    file.persist(path)?;
    println!("{}", "输入方案安装完成。".bright_green());
    Ok(())
}
