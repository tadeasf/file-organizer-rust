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
    command: Commands,
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
        Ok(())
    }
} 