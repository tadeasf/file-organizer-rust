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
    #[allow(unused)]
    fn is_recursive(&self) -> bool;
    
    /// Get the input directory for the operation
    #[allow(unused)]
    fn get_input_dir(&self) -> Option<&PathBuf>;
    
    /// Set the input directory for the operation
    #[allow(unused)]
    fn set_input_dir(&mut self, dir: PathBuf);
    
    /// Process a single file
    fn process_file(&self, file: &PathBuf) -> Result<()>;
    
    /// Create necessary directories for the operation
    #[allow(unused)]
    fn create_directories(&self, base_dir: &PathBuf) -> Result<()>;
} 