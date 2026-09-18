use anyhow::{Context, Result};
use colored::Colorize;
use octocrab::models::repos::{Asset, Release};
use semver::Version;
use std::{fs, path::Path};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::installed_detector::InstalledSchema;
use crate::network::Network;
use crate::options::Schema;
use crate::print_err;
use crate::workflow::{ApplyFuture, Module};

pub struct Install {
    pub schema: Schema,
    pub latest: LatestSchema,
    pub previous: Option<Schema>,
}

impl Module for Install {
    fn info(&self) {
        info_install(self.schema);
    }

    fn warn(&self, root: &Path) -> Result<()> {
        warn_install(root)
    }

    fn apply<'a>(self: Box<Self>, downloader: &'a Network, root: &'a Path) -> ApplyFuture<'a> {
        Box::pin(async move {
            update(downloader, root, self.schema, self.latest)
                .await
                .context("更新输入方案失败")?;
            if let Some(previous) = self.previous {
                cleanup(root, previous, self.schema);
            }
            Ok(())
        })
    }
}

#[derive(Clone, PartialEq)]
pub struct LatestSchema {
    pub release: Release,
}

pub async fn get_latest(prerelease: bool) -> Result<LatestSchema> {
    let octocrab = octocrab::instance();
    let repo = octocrab.repos("Fidelxyz", "rime-wanxiang-slim");
    let release = if prerelease {
        repo.releases()
            .list()
            .per_page(1)
            .send()
            .await?
            .items
            .first()
            .context("Release 列表为空")?
            .clone()
    } else {
        repo.releases().get_latest().await?
    };
    Ok(LatestSchema { release })
}

pub fn check_update(installed: &InstalledSchema, latest: &LatestSchema) -> Result<bool> {
    let installed_version = Version::parse(&installed.version)?;
    let latest_version = Version::parse(latest.release.tag_name.trim_start_matches('v'))?;
    Ok(latest_version > installed_version)
}

fn info_install(schema: Schema) {
    println!("{} 将安装输入方案：", "==>".bright_green());
    println!("  方案：{}", schema.to_string().bright_cyan());
    if let Schema::Pro(aux) = schema {
        println!("  辅助码方案：{}", aux.unwrap().to_string().bright_cyan());
    }
}

fn warn_install(root: &Path) -> Result<()> {
    println!(
        "{}",
        format!("将安装输入方案至目录：{}", root.display()).bright_yellow()
    );
    if root.exists() && root.read_dir()?.next().is_some() {
        println!("{}", "该目录下方案文件与子文件夹将被覆盖。".bright_yellow());
    }
    Ok(())
}

async fn update(
    downloader: &Network,
    root: &Path,
    schema: Schema,
    latest: LatestSchema,
) -> Result<()> {
    let asset = asset(schema, latest)?;
    let downloaded = downloader
        .download(&asset)
        .await
        .context("输入方案下载失败")?;
    install(downloaded, root).context("输入方案安装失败")?;
    Ok(())
}

fn asset(schema: Schema, latest: LatestSchema) -> Result<Asset> {
    latest
        .release
        .assets
        .into_iter()
        .find(|asset| {
            asset.name
                == match schema {
                    Schema::Base => String::from("rime-wanxiang-base.zip"),
                    Schema::Pro(Some(aux)) => format!("rime-wanxiang-{}-fuzhu.zip", aux.code()),
                    Schema::Pro(None) => panic!(),
                }
        })
        .context("Release 中未找到 Asset")
}

fn install(file: NamedTempFile, path: &Path) -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(&temp)?;

    for entry in temp.path().read_dir()? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src = entry.path();
        let dst = path.join(entry.file_name());

        if file_type.is_dir() {
            if dst.exists() {
                fs::remove_dir_all(&dst)
                    .with_context(|| format!("无法删除目录 {}", dst.display()))?;
            }
            fs::rename(&src, &dst).or_else(|_| -> Result<_> {
                for entry in WalkDir::new(&src) {
                    let entry = entry?;
                    let entry_src = entry.path();
                    let entry_dst = dst.join(entry_src.strip_prefix(&src).unwrap());
                    if entry.file_type().is_dir() {
                        fs::create_dir_all(&entry_dst)
                            .with_context(|| format!("无法创建目录 {}", entry_dst.display()))?;
                    } else {
                        fs::copy(entry_src, &entry_dst).with_context(|| {
                            format!(
                                "无法复制文件 {} 至 {}",
                                entry_src.display(),
                                entry_dst.display()
                            )
                        })?;
                    }
                }
                Ok(())
            })?;
        } else if file_type.is_file() {
            fs::rename(&src, &dst)
                .or_else(|_| -> Result<_> {
                    fs::copy(&src, &dst)?;
                    Ok(())
                })
                .with_context(|| format!("无法复制文件 {} 至 {}", src.display(), dst.display()))?;
        }
    }

    println!("{}", "输入方案安装完成。".bright_green());
    Ok(())
}

fn cleanup(root: &Path, old: Schema, new: Schema) {
    if std::mem::discriminant(&old) != std::mem::discriminant(&new) {
        let old_schema = root.join(format!("{}.schema.yaml", old.schema_id()));
        fs::remove_file(&old_schema)
            .with_context(|| format!("无法移除旧方案文件 {}", old_schema.display()))
            .unwrap_or_else(print_err);
    }
}
