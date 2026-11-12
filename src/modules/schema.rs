use anyhow::{Context, Result};
use colored::Colorize;
use octocrab::models::repos::{Asset, Release};
use regex::regex;
use semver::Version;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;
use tempfile::{NamedTempFile, TempDir};
use zip::ZipArchive;

use crate::changelog::breaking_changes;
use crate::error::print_err;
use crate::network::Network;
use crate::options::{AuxCode, Schema};
use crate::workflow::{Module, PrepareFuture, PreparedModule};
use crate::yaml::Document;

#[derive(Clone)]
pub struct InstalledSchema {
    pub schema: Schema,
    pub version: String,
}

pub fn detect(root: &Path) -> Vec<InstalledSchema> {
    let default_path = root.join("default.yaml");
    if !default_path.exists() {
        return vec![];
    }
    let default_document = match Document::open(&default_path)
        .with_context(|| format!("无法读取配置文件 {}", default_path.display()))
    {
        Ok(document) => document,
        Err(e) => {
            print_err(&e);
            return vec![];
        }
    };
    let schema_list = default_document.values(&["schema_list", "schema"]);

    let mut installed = vec![];
    for schema in Schema::iter() {
        let schema_id = schema.schema_id();
        if !schema_list
            .iter()
            .any(|field| field.data.as_str() == Some(schema_id))
        {
            continue;
        }

        let path = root.join(format!("{schema_id}.schema.yaml"));
        let schema_document = match Document::open(&path)
            .with_context(|| format!("无法读取方案文件 {}", path.display()))
        {
            Ok(document) => document,
            Err(e) => {
                print_err(&e);
                continue;
            }
        };
        let version = match schema_document
            .required(&["schema", "version"])
            .and_then(|field| {
                field
                    .data
                    .as_str()
                    .context("方案版本字段不是字符串")
                    .map(str::to_owned)
            })
            .with_context(|| format!("无法读取方案版本 {}", path.display()))
        {
            Ok(version) => version,
            Err(e) => {
                print_err(&e);
                continue;
            }
        };

        // Read aux code from meta.lua if it exists
        let aux = match schema {
            Schema::Base => None,
            Schema::Pro(_) => fs::read_to_string(root.join("lua/meta.lua"))
                .ok()
                .and_then(|meta| {
                    regex!(r#"(?m)^\s*M\.AUX_CODE\s*=\s*["']([^"']+)["']"#)
                        .captures(&meta)
                        .and_then(|c| AuxCode::iter().find(|value| value.code() == &c[1]))
                }),
        };

        installed.push(InstalledSchema {
            schema: match schema {
                Schema::Base => Schema::Base,
                Schema::Pro(_) => Schema::Pro(aux),
            },
            version,
        });
    }
    installed
}

pub struct Install {
    pub schema: Schema,
    pub latest: LatestSchema,
}

impl Module for Install {
    fn info(&self) {
        info_install(self.schema);
    }

    fn prepare<'a>(self: Box<Self>, downloader: &'a Network, root: &'a Path) -> PrepareFuture<'a> {
        Box::pin(async move {
            let file = download(downloader, root, self.schema, &self.latest)
                .await
                .context("输入方案下载失败")?;
            let package = extract(file).context("输入方案解压失败")?;
            let plan = InstallPlan::new(root, package)?;
            Ok(Box::new(PreparedInstall { plan }) as Box<dyn PreparedModule>)
        })
    }
}

struct PreparedInstall {
    plan: InstallPlan,
}

impl PreparedModule for PreparedInstall {
    fn warn(&self, root: &Path) {
        warn_install(root, &self.plan);
    }

    fn apply(self: Box<Self>, root: &Path) -> Result<()> {
        install(root, &self.plan).context("输入方案安装失败")
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

pub fn check_update(
    installed: &InstalledSchema,
    latest: &LatestSchema,
    prerelease: bool,
) -> Result<bool> {
    let installed_version = Version::parse(&installed.version)?;
    let latest_version = Version::parse(latest.release.tag_name.trim_start_matches('v'))?;

    // Report update available if the installed version is a prerelease while the user wants stable
    // releases only
    if !prerelease && !installed_version.pre.is_empty() {
        return Ok(true);
    }
    Ok(latest_version > installed_version)
}

pub async fn get_breaking_changes(
    installed: &InstalledSchema,
    latest: &LatestSchema,
) -> Result<Vec<(Version, String)>> {
    let installed_version = Version::parse(&installed.version)?;
    let latest_version = Version::parse(latest.release.tag_name.trim_start_matches('v'))?;
    if latest_version <= installed_version {
        return Ok(vec![]);
    }

    let changelog = octocrab::instance()
        .repos("Fidelxyz", "rime-wanxiang-slim")
        .get_content()
        .path("CHANGELOG.md")
        .r#ref(&latest.release.tag_name)
        .send()
        .await?
        .items
        .first()
        .and_then(octocrab::models::repos::Content::decoded_content)
        .context("CHANGELOG.md 内容为空")?;
    breaking_changes(&changelog, &installed_version, &latest_version)
}

fn info_install(schema: Schema) {
    println!("{} 将安装输入方案：", "==>".bright_green());
    println!("  方案：{}", schema.to_string().bright_cyan());
    if let Schema::Pro(aux) = schema {
        println!("  辅助码方案：{}", aux.unwrap().to_string().bright_cyan());
    }
}

async fn download(
    downloader: &Network,
    root: &Path,
    schema: Schema,
    latest: &LatestSchema,
) -> Result<NamedTempFile> {
    downloader.download(root, asset(schema, latest)?).await
}

fn asset(schema: Schema, latest: &LatestSchema) -> Result<&Asset> {
    latest
        .release
        .assets
        .iter()
        .find(|asset| {
            asset.name
                == match schema {
                    Schema::Base => String::from("rime-wanxiang-base.zip"),
                    Schema::Pro(Some(aux)) => format!("rime-wanxiang-{}-fuzhu.zip", aux.code()),
                    Schema::Pro(None) => unreachable!(),
                }
        })
        .context("Release 中未找到 Asset")
}

fn extract(file: NamedTempFile) -> Result<TempDir> {
    let extracted = tempfile::tempdir_in(file.path().parent().unwrap())?;
    ZipArchive::new(file)?.extract(&extracted)?;
    Ok(extracted)
}

struct InstallPlan {
    package: TempDir,
    overwritten_dirs: Vec<PathBuf>,
    overwritten_files: Vec<PathBuf>,
    stale_files: Vec<PathBuf>,
}

impl InstallPlan {
    fn new(root: &Path, package: TempDir) -> Result<Self> {
        let mut overwritten_dirs = vec![];
        let mut overwritten_files = vec![];
        let mut stale_files = vec![];

        for entry in root.read_dir()? {
            let entry = entry?;
            let package_path = package.path().join(entry.file_name());
            if package_path.is_dir() {
                overwritten_dirs.push(entry.path());
            } else if package_path.exists() {
                overwritten_files.push(entry.path());
            } else if is_stale_file(&entry.file_name()) {
                stale_files.push(entry.path());
            }
        }

        overwritten_dirs.sort_unstable();
        overwritten_files.sort_unstable();
        stale_files.sort_unstable();

        Ok(Self {
            package,
            overwritten_dirs,
            overwritten_files,
            stale_files,
        })
    }
}

fn is_stale_file(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let lowercase_name = name.to_ascii_lowercase();
    lowercase_name.starts_with("wanxiang")
        && Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml"))
        && !lowercase_name.ends_with(".custom.yaml")
}

fn warn_install(root: &Path, plan: &InstallPlan) {
    eprintln!(
        "{}",
        format!("将安装输入方案至目录：{}", root.display()).bright_yellow()
    );
    for path in &plan.overwritten_dirs {
        eprintln!(
            "{}",
            format!("将覆盖文件夹：{}", path.display()).bright_yellow()
        );
    }
    for path in &plan.overwritten_files {
        eprintln!(
            "{}",
            format!("将覆盖文件：{}", path.display()).bright_yellow()
        );
    }
    for path in &plan.stale_files {
        eprintln!(
            "{}",
            format!("将删除文件：{}", path.display()).bright_yellow()
        );
    }
}

fn install(root: &Path, plan: &InstallPlan) -> Result<()> {
    remove_old_files(plan)?;

    for entry in plan.package.path().read_dir()? {
        let src = entry?.path();
        let dst = root.join(src.file_name().unwrap());
        fs::rename(&src, &dst)
            .with_context(|| format!("无法移动 {} 至 {}", src.display(), dst.display()))?;
    }

    println!("{}", "输入方案安装完成。".bright_green());
    Ok(())
}

fn remove_old_files(plan: &InstallPlan) -> Result<()> {
    for dir in &plan.overwritten_dirs {
        fs::remove_dir_all(dir).with_context(|| format!("无法删除目录 {}", dir.display()))?;
    }
    for file in &plan.stale_files {
        fs::remove_file(file).with_context(|| format!("无法删除文件 {}", file.display()))?;
    }
    Ok(())
}
