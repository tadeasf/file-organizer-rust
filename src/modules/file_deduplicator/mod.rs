use anyhow::Result;
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, Select, MultiSelect};
use sha2::{Sha256, Digest};
use std::{collections::HashMap, fs, path::PathBuf, io::Read};
use walkdir::WalkDir;

use crate::utils::{create_spinner, get_directory_from_user};
use crate::modules::base::FileOrganizer;

pub struct FileDeduplicator {
    recursive: bool,
    input_dir: Option<PathBuf>,
    duplicate_action: Option<DuplicateAction>,
    hash_method: Option<HashMethod>,
    duplicates_dir: Option<PathBuf>,
    file_hashes: HashMap<String, Vec<PathBuf>>,
}

#[derive(Clone, Copy)]
enum DuplicateAction {
    Delete,
    Move,
    Report,
}

#[derive(Clone, Copy)]
enum HashMethod {
    Sha256,
    QuickHash,  // First 1MB + file size
}

#[async_trait]
impl FileOrganizer for FileDeduplicator {
    fn new(recursive: bool) -> Self {
        Self {
            recursive,
            input_dir: None,
            duplicate_action: None,
            hash_method: None,
            duplicates_dir: None,
            file_hashes: HashMap::new(),
        }
    }

    async fn run(&self) -> Result<()> {
        let input_dir = get_directory_from_user("Enter directory to scan for duplicates")?;
        
        // Select hash method
        let hash_options = vec!["SHA-256 (Accurate)", "Quick Hash (Fast)"];
        let hash_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select hash method")
            .items(&hash_options)
            .default(0)
            .interact()?;

        let hash_method = match hash_selection {
            0 => HashMethod::Sha256,
            1 => HashMethod::QuickHash,
            _ => unreachable!(),
        };

        // Select action for duplicates
        let action_options = vec!["Delete duplicates", "Move to separate directory", "Generate report only"];
        let action_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What to do with duplicates?")
            .items(&action_options)
            .default(0)
            .interact()?;

        let duplicate_action = match action_selection {
            0 => DuplicateAction::Delete,
            1 => DuplicateAction::Move,
            2 => DuplicateAction::Report,
            _ => unreachable!(),
        };

        // Create duplicates directory if needed
        let duplicates_dir = if matches!(duplicate_action, DuplicateAction::Move) {
            let dir = input_dir.join("duplicates");
            fs::create_dir_all(&dir)?;
            Some(dir)
        } else {
            None
        };

        // Set up state
        let mut this = Self {
            recursive: self.recursive,
            input_dir: Some(input_dir.clone()),
            duplicate_action: Some(duplicate_action),
            hash_method: Some(hash_method),
            duplicates_dir,
            file_hashes: HashMap::new(),
        };

        let spinner = create_spinner("Scanning for duplicates...");
        
        // First pass: collect all file hashes
        this.collect_file_hashes()?;

        // Second pass: handle duplicates
        let mut total_duplicates = 0;
        let mut total_space_saved = 0;
        
        for (_hash, paths) in this.file_hashes.iter() {
            if paths.len() > 1 {
                let duplicates = &paths[1..]; // Keep the first occurrence
                total_duplicates += duplicates.len();
                
                for duplicate in duplicates {
                    let file_size = fs::metadata(duplicate)?.len();
                    total_space_saved += file_size;

                    match this.duplicate_action.unwrap() {
                        DuplicateAction::Delete => {
                            fs::remove_file(duplicate)?;
                        }
                        DuplicateAction::Move => {
                            if let Some(ref dup_dir) = this.duplicates_dir {
                                let new_path = dup_dir.join(duplicate.file_name().unwrap());
                                fs::rename(duplicate, new_path)?;
                            }
                        }
                        DuplicateAction::Report => {
                            println!("Duplicate found: {}", duplicate.display());
                            println!("  Original: {}", paths[0].display());
                            println!("  Size: {} bytes", file_size);
                        }
                    }
                }
            }
        }

        let action_msg = match this.duplicate_action.unwrap() {
            DuplicateAction::Delete => "deleted",
            DuplicateAction::Move => "moved",
            DuplicateAction::Report => "found",
        };

        spinner.finish_with_message(format!(
            "Found and {} {} duplicate files (total {} bytes)",
            action_msg,
            total_duplicates,
            total_space_saved
        ));

        Ok(())
    }

    fn is_recursive(&self) -> bool {
        self.recursive
    }

    fn get_input_dir(&self) -> Option<&PathBuf> {
        self.input_dir.as_ref()
    }

    fn set_input_dir(&mut self, dir: PathBuf) {
        self.input_dir = Some(dir);
    }

    fn process_file(&self, file: &PathBuf) -> Result<()> {
        let hash = match self.hash_method.unwrap() {
            HashMethod::Sha256 => self.calculate_sha256(file)?,
            HashMethod::QuickHash => self.calculate_quick_hash(file)?,
        };

        if let Some(paths) = self.file_hashes.get(&hash) {
            match self.duplicate_action.unwrap() {
                DuplicateAction::Delete => {
                    fs::remove_file(file)?;
                }
                DuplicateAction::Move => {
                    if let Some(ref dup_dir) = self.duplicates_dir {
                        let new_path = dup_dir.join(file.file_name().unwrap());
                        fs::rename(file, new_path)?;
                    }
                }
                DuplicateAction::Report => {
                    println!("Duplicate found: {}", file.display());
                    println!("  Original: {}", paths[0].display());
                }
            }
        }
        Ok(())
    }

    fn create_directories(&self, base_dir: &PathBuf) -> Result<()> {
        if matches!(self.duplicate_action, Some(DuplicateAction::Move)) {
            fs::create_dir_all(base_dir.join("duplicates"))?;
        }
        Ok(())
    }
}

impl FileDeduplicator {
    fn collect_file_hashes(&mut self) -> Result<()> {
        let input_dir = self.input_dir.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Input directory not set")
        })?;

        let walker = if self.recursive {
            WalkDir::new(input_dir)
        } else {
            WalkDir::new(input_dir).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path().to_path_buf();
            let hash = match self.hash_method.unwrap() {
                HashMethod::Sha256 => self.calculate_sha256(&path)?,
                HashMethod::QuickHash => self.calculate_quick_hash(&path)?,
            };

            self.file_hashes.entry(hash).or_insert_with(Vec::new).push(path);
        }

        Ok(())
    }

    fn calculate_sha256(&self, file: &PathBuf) -> Result<String> {
        let mut file = fs::File::open(file)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];

        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn calculate_quick_hash(&self, file: &PathBuf) -> Result<String> {
        let mut file = fs::File::open(file)?;
        let metadata = file.metadata()?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024 * 1024]; // 1MB buffer

        // Hash file size
        hasher.update(metadata.len().to_string().as_bytes());

        // Hash first 1MB
        let count = file.read(&mut buffer)?;
        hasher.update(&buffer[..count]);

        Ok(format!("{:x}", hasher.finalize()))
    }
} 