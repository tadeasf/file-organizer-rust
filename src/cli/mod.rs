use anyhow::Result;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};

use crate::modules::{
    directory_flattener::DirectoryFlattener,
    image_optimizer::ImageOptimizer,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    ImageOptimize {
        #[arg(short, long)]
        recursive: bool,
    },
    DirectoryFlatten,
}

impl Cli {
    pub fn new() -> Result<Self> {
        Ok(Self::parse())
    }

    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Some(Commands::ImageOptimize { recursive }) => {
                let optimizer = ImageOptimizer::new(*recursive);
                optimizer.run().await?;
            }
            Some(Commands::DirectoryFlatten) => {
                let flattener = DirectoryFlattener::new();
                flattener.run().await?;
            }
            None => {
                let options = vec!["Image Optimizer", "Directory Flattener"];
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select operation")
                    .items(&options)
                    .default(0)
                    .interact()?;

                match selection {
                    0 => {
                        let recursive = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Run recursively?")
                            .items(&["No", "Yes"])
                            .default(0)
                            .interact()?;
                        let optimizer = ImageOptimizer::new(recursive == 1);
                        optimizer.run().await?;
                    }
                    1 => {
                        let flattener = DirectoryFlattener::new();
                        flattener.run().await?;
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }
} 