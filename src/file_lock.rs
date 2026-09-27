use anyhow::{Context, Result, ensure};
use colored::Colorize;
use inquire::Confirm;
use restart_manager::{AffectedApplications, RestartSession, ShutdownOptions};
use std::path::Path;

use crate::error::{UserCancelled, print_err};

pub fn release_file_locks(path: &Path) -> Result<()> {
    let result = match query_file_locks(path).context("查询文件占用进程失败") {
        Ok(None) => Ok(()),
        Ok(Some((session, applications))) => {
            eprintln!(
                "{}",
                format!("以下程序正在占用 {}：", path.display()).bright_yellow()
            );
            for application in &applications {
                eprintln!(
                    "{}",
                    format!("- {}", application.display_name().display()).bright_yellow()
                );
            }
            ensure!(
                Confirm::new("是否终止以上程序并继续安装？")
                    .with_default(true)
                    .prompt()?,
                UserCancelled
            );

            shutdown_applications(session).context("关闭文件占用进程失败")
        }
        Err(e) => Err(e),
    };

    if let Err(e) = result {
        print_err(&e);
        eprintln!(
            "{}",
            format!("请手动关闭所有占用 {} 的程序。", path.display()).bright_yellow()
        );
        ensure!(
            Confirm::new("是否已关闭占用程序并继续安装？")
                .with_default(true)
                .prompt()?,
            UserCancelled
        );
    }

    Ok(())
}

fn query_file_locks(path: &Path) -> Result<Option<(RestartSession, AffectedApplications)>> {
    let mut session = RestartSession::new()?;
    session.register_files([path])?;
    let applications = session.affected_applications()?;
    Ok(if applications.is_empty() {
        None
    } else {
        Some((session, applications))
    })
}

fn shutdown_applications(session: RestartSession) -> Result<()> {
    Ok(session
        .shutdown_with_options(ShutdownOptions::new().with_force_if_hung(true))
        .shutdown_outcome()
        .clone()
        .into_result()?)
}
