use anyhow::{Context, Result};
use colored::Colorize;
use octocrab::models::repos::Asset;
use std::fs::File;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::digest;
use crate::downloader::Downloader;

pub const GRAMMAR_NAME: &str = "wanxiang-lts-zh-hans.gram";

pub async fn get_latest() -> Result<Asset> {
    octocrab::instance()
        .repos("amzxyz", "RIME-LMDG")
        .releases()
        .get_by_tag("LTS")
        .await?
        .assets
        .into_iter()
        .find(|asset| asset.name == GRAMMAR_NAME)
        .context("Release 中未找到语法模型 Asset")
}

pub fn check_update(path: &Path, latest: &Asset) -> Result<bool> {
    let installed = File::open(path)?;
    Ok(!digest::matches(&installed, latest)?)
}

pub fn prompt_install(root: &Path) {
    println!(
        "{}",
        format!("将安装语法模型至：{}", root.join(GRAMMAR_NAME).display()).bright_yellow()
    );
    if root.join(GRAMMAR_NAME).exists() {
        println!("{}", "原有文件将被覆盖。".bright_yellow());
    }
}

pub async fn update(downloader: &Downloader, root: &Path, asset: &Asset) -> Result<()> {
    let downloaded = downloader
        .download(asset)
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
