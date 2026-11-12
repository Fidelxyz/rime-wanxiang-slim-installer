use anyhow::{Result, ensure};
use colored::Colorize;
use futures_util::future::try_join_all;
use inquire::Confirm;
use std::{future::Future, path::Path, pin::Pin};

use crate::{error::UserCancelled, network::Network};

pub type PrepareFuture<'a> = Pin<Box<dyn Future<Output = Result<Box<dyn PreparedModule>>> + 'a>>;

pub trait Module {
    fn info(&self);

    fn prepare<'a>(self: Box<Self>, downloader: &'a Network, root: &'a Path) -> PrepareFuture<'a>;
}

pub trait PreparedModule {
    fn warn(&self, root: &Path);

    fn apply(self: Box<Self>, root: &Path) -> Result<()>;
}

#[derive(Default)]
pub struct Workflow {
    modules: Vec<Box<dyn Module>>,
}

impl Workflow {
    pub fn register(&mut self, module: impl Module + 'static) {
        self.modules.push(Box::new(module));
    }

    pub async fn execute(self, downloader: &Network, root: &Path) -> Result<()> {
        if self.modules.is_empty() {
            return Ok(());
        }

        for module in &self.modules {
            module.info();
        }
        let prepared = try_join_all(
            self.modules
                .into_iter()
                .map(|module| module.prepare(downloader, root)),
        )
        .await?;
        for module in &prepared {
            module.warn(root);
        }
        ensure!(Self::prompt_confirm()?, UserCancelled);
        for module in prepared {
            module.apply(root)?;
        }
        Self::prompt_finish();
        Ok(())
    }

    fn prompt_confirm() -> Result<bool> {
        Ok(Confirm::new("是否继续？").with_default(true).prompt()?)
    }

    fn prompt_finish() {
        println!("{}", "完成，请重新部署。".bright_green());
    }
}
