use anyhow::Result;
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use octocrab::{self, models::repos::Asset};
use reqwest::{Client, StatusCode, header};
use std::{
    fs,
    io::{self, Seek, Write},
    path::Path,
    time::Duration,
};
use tempfile::NamedTempFile;

use crate::digest;

const INSTALLER_DIR: &str = "./installer";

pub struct Network {
    client: Client,
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
        Ok(Self { client })
    }

    pub async fn check_update(&self, path: &Path, asset: &Asset) -> Result<bool> {
        let modified: DateTime<Utc> = fs::metadata(path)?.modified()?.into();
        let response = self
            .client
            .get(asset.browser_download_url.clone())
            .header(
                header::IF_MODIFIED_SINCE,
                modified.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
            )
            .send()
            .await?
            .error_for_status()?;
        Ok(response.status() != StatusCode::NOT_MODIFIED)
    }

    pub async fn download(&self, asset: &Asset) -> Result<NamedTempFile> {
        fs::create_dir_all(INSTALLER_DIR)?;
        let mut tempfile = NamedTempFile::new_in(INSTALLER_DIR)?;

        println!("下载 {}", asset.name);

        let mut response = self
            .client
            .get(asset.browser_download_url.clone())
            .send()
            .await?
            .error_for_status()?;

        let modified = response
            .headers()
            .get(header::LAST_MODIFIED)
            .map(|value| -> Result<_> { Ok(DateTime::parse_from_rfc2822(value.to_str()?)?.into()) })
            .transpose()?;

        let total = response.content_length();
        let progress = ProgressBar::new(total.unwrap_or(0)).with_style(
            ProgressStyle::with_template("[{percent:>3}%] {wide_bar} {bytes} / {total_bytes}")
                .unwrap(),
        );
        while let Some(chunk) = response.chunk().await? {
            tempfile.write_all(&chunk)?;
            progress.inc(chunk.len() as u64);
        }
        progress.finish();

        tempfile.flush()?;
        tempfile.seek(io::SeekFrom::Start(0))?;
        digest::verify(tempfile.as_file(), asset)?;
        if let Some(modified) = modified {
            tempfile.as_file().set_modified(modified)?;
        }

        progress.finish_and_clear();
        Ok(tempfile)
    }
}
