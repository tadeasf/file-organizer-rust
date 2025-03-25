use anyhow::Result;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};

use crate::modules::{
    directory_flattener::DirectoryFlattener,
    image_optimizer::ImageOptimizer,
    file_deduplicator::FileDeduplicator,
    file_categorizer::FileCategorizer,
    archive_manager::ArchiveManager,
    base::FileOrganizer,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Categorize files based on type and date
    Categorize { recursive: bool },
    /// Flatten directory structure
    DirectoryFlatten { recursive: bool },
    /// Optimize images
    ImageOptimize { recursive: bool },
    /// Find and handle duplicate files
    Deduplicate { recursive: bool },
    /// Manage archives (create, extract, update, split)
    Archive { recursive: bool },
}

impl Cli {
    pub fn new() -> Result<Self> {
        Ok(Self::parse())
    }

    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Some(cmd) => {
                match cmd {
                    Commands::Categorize { recursive } => {
                        let organizer = FileCategorizer::new(*recursive);
                        organizer.run().await?;
                    }
                    Commands::DirectoryFlatten { recursive } => {
                        let organizer = DirectoryFlattener::new(*recursive);
                        organizer.run().await?;
                    }
                    Commands::ImageOptimize { recursive } => {
                        let organizer = ImageOptimizer::new(*recursive);
                        organizer.run().await?;
                    }
                    Commands::Deduplicate { recursive } => {
                        let organizer = FileDeduplicator::new(*recursive);
                        organizer.run().await?;
                    }
                    Commands::Archive { recursive } => {
                        let organizer = ArchiveManager::new(*recursive);
                        organizer.run().await?;
                    }
                }
            }
            None => {
                // Interactive mode
                let options = vec![
                    "Categorize files",
                    "Flatten directory",
                    "Optimize images",
                    "Find duplicates",
                    "Manage archives",
                ];
                
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select operation")
                    .items(&options)
                    .default(0)
                    .interact()?;

                let recursive = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Process subdirectories recursively?")
                    .default(true)
                    .interact()?;

                match selection {
                    0 => {
                        let organizer = FileCategorizer::new(recursive);
                        organizer.run().await?;
                    }
                    1 => {
                        let organizer = DirectoryFlattener::new(recursive);
                        organizer.run().await?;
                    }
                    2 => {
                        let organizer = ImageOptimizer::new(recursive);
                        organizer.run().await?;
                    }
                    3 => {
                        let organizer = FileDeduplicator::new(recursive);
                        organizer.run().await?;
                    }
                    4 => {
                        let organizer = ArchiveManager::new(recursive);
                        organizer.run().await?;
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }
} 