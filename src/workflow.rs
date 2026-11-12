use anyhow::{Result, ensure};
use colored::Colorize;
use inquire::Confirm;
use std::{future::Future, path::Path, pin::Pin};

use crate::{error::UserCancelled, network::Network};

pub type ModuleFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + 'a>>;

pub trait Module {
    fn info(&self);

    fn prepare<'a>(&'a mut self, _downloader: &'a Network, _root: &'a Path) -> ModuleFuture<'a> {
        Box::pin(async { Ok(()) })
    }

    fn warn(&self, root: &Path) -> Result<()>;

    fn apply<'a>(self: Box<Self>, downloader: &'a Network, root: &'a Path) -> ModuleFuture<'a>;
}

#[derive(Default)]
pub struct Workflow {
    modules: Vec<Box<dyn Module>>,
}

impl Workflow {
    pub fn register(&mut self, module: impl Module + 'static) {
        self.modules.push(Box::new(module));
    }

    pub async fn execute(mut self, downloader: &Network, root: &Path) -> Result<()> {
        if self.modules.is_empty() {
            return Ok(());
        }

        for module in &self.modules {
            module.info();
        }
        for module in &mut self.modules {
            module.prepare(downloader, root).await?;
        }
        for module in &self.modules {
            module.warn(root)?;
        }
        ensure!(Self::prompt_confirm()?, UserCancelled);
        for module in self.modules {
            module.apply(downloader, root).await?;
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
