use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use octocrab::models::repos::Asset;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

use crate::network::Network;
use crate::workflow::{Module, PrepareFuture, PreparedModule};

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

    fn prepare<'a>(self: Box<Self>, downloader: &'a Network, root: &'a Path) -> PrepareFuture<'a> {
        Box::pin(async move {
            let file = download(downloader, root, &self.latest)
                .await
                .context("语法模型下载失败")?;
            Ok(Box::new(PreparedInstall { file }) as Box<dyn PreparedModule>)
        })
    }
}

struct PreparedInstall {
    file: NamedTempFile,
}

impl PreparedModule for PreparedInstall {
    fn warn(&self, root: &Path) {
        warn_install(root);
    }

    fn apply(self: Box<Self>, root: &Path) -> Result<()> {
        install(self.file, &root.join(GRAMMAR_NAME)).context("语法模型安装失败")
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

pub fn check_update(installed: &InstalledGrammar, latest: &LatestGrammar) -> Result<bool> {
    let modified: DateTime<Utc> = fs::metadata(&installed.path)?.modified()?.into();
    Ok(latest.asset.updated_at > modified)
}

fn info_install() {
    println!("{} 将安装语法模型：", "==>".green());
    println!("  语法模型：{}", GRAMMAR_NAME.cyan());
}

fn warn_install(root: &Path) {
    eprintln!(
        "{}",
        format!("将安装语法模型至：{}", root.join(GRAMMAR_NAME).display()).yellow()
    );
    if root.join(GRAMMAR_NAME).exists() {
        eprintln!("{}", "原有文件将被覆盖。".yellow());
    }
}

async fn download(
    downloader: &Network,
    root: &Path,
    latest: &LatestGrammar,
) -> Result<NamedTempFile> {
    downloader.download(root, &latest.asset).await
}

fn install(file: NamedTempFile, path: &Path) -> Result<()> {
    #[cfg(windows)]
    if path.exists() {
        crate::file_lock::release_file_locks(path)?;
    }

    file.persist(path)?;
    println!("{}", "语法模型安装完成。".green());
    Ok(())
}
