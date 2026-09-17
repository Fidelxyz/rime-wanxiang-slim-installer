use anyhow::{Context, Result};
use colored::Colorize;
use octocrab::models::repos::Asset;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::network::Network;

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

pub async fn check_update(downloader: &Network, path: &Path, latest: &Asset) -> Result<bool> {
    downloader.check_update(path, latest).await
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

pub async fn update(downloader: &Network, root: &Path, asset: &Asset) -> Result<()> {
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
