use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait FileOrganizer {
    /// Initialize a new instance of the organizer
    fn new(recursive: bool) -> Self where Self: Sized;
    
    /// Run the organization process
    async fn run(&self) -> Result<()>;
    
    /// Whether the organizer operates recursively on subdirectories
    fn is_recursive(&self) -> bool;
    
    /// Get the input directory for the operation
    fn get_input_dir(&self) -> Option<&PathBuf>;
    
    /// Set the input directory for the operation
    fn set_input_dir(&mut self, dir: PathBuf);
    
    /// Process a single file
    fn process_file(&self, file: &PathBuf) -> Result<()>;
    
    /// Create necessary directories for the operation
    fn create_directories(&self, base_dir: &PathBuf) -> Result<()>;
} 