use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use octocrab::{self, models::repos::Asset};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

use crate::INSTALLER_DIR;
use crate::digest;

pub struct Network {
    client: Client,
    progress: MultiProgress,
}

impl Network {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!(
                "rime-wanxiang-slim-installer/",
                env!("CARGO_PKG_VERSION")
            ))
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_hours(1))
            .build()?;
        Ok(Self {
            client,
            progress: MultiProgress::new(),
        })
    }

    pub async fn download(&self, root: &Path, asset: &Asset) -> Result<NamedTempFile> {
        let installer_dir = root.join(INSTALLER_DIR);
        fs::create_dir_all(&installer_dir)
            .with_context(|| format!("无法创建目录 {}", installer_dir.display()))?;
        let tempfile = NamedTempFile::new_in(&installer_dir)
            .with_context(|| format!("无法创建临时文件于目录 {}", installer_dir.display()))?;
        let mut file = tokio::fs::File::from_std(tempfile.as_file().try_clone()?);

        let mut response = self
            .client
            .get(asset.browser_download_url.clone())
            .send()
            .await?
            .error_for_status()?;

        let mut hasher = Sha256::new();

        let total = response.content_length();
        let progress = self.progress.add(
            ProgressBar::new(total.unwrap_or(0))
                .with_style(
                    ProgressStyle::with_template(
                        "{prefix} [{percent:>3}%] {wide_bar} {bytes} / {total_bytes}",
                    )
                    .unwrap(),
                )
                .with_prefix(format!("下载 {}", asset.name)),
        );
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            progress.inc(chunk.len() as u64);
        }
        progress.finish();

        file.flush().await?;
        digest::verify(&hasher.finalize(), asset)?;
        tempfile.as_file().set_modified(asset.updated_at.into())?;

        progress.finish_and_clear();
        Ok(tempfile)
    }
}
