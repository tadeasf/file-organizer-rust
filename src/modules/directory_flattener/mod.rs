use anyhow::Result;
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, Select};
use std::{collections::HashMap, fs, path::PathBuf};
use walkdir::WalkDir;

use crate::utils::{create_spinner, get_directory_from_user};
use crate::modules::base::FileOrganizer;

pub struct DirectoryFlattener {
    recursive: bool,
    input_dir: Option<PathBuf>,
    handle_duplicates: Option<DuplicateHandling>,
}

#[derive(Clone, Copy)]
enum DuplicateHandling {
    Rename,
    Skip,
}

#[async_trait]
impl FileOrganizer for DirectoryFlattener {
    fn new(_recursive: bool) -> Self {
        Self {
            recursive: true,  // Directory flattener is always recursive
            input_dir: None,
            handle_duplicates: Some(DuplicateHandling::Rename),
        }
    }

    async fn run(&self) -> Result<()> {
        let input_dir = get_directory_from_user("Enter directory to flatten")?;
        
        let options = vec!["Rename duplicates", "Skip duplicates"];
        let handle_duplicates = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How to handle duplicate filenames?")
            .items(&options)
            .default(0)
            .interact()?;

        let handle_duplicates = match handle_duplicates {
            0 => DuplicateHandling::Rename,
            1 => DuplicateHandling::Skip,
            _ => unreachable!(),
        };

        let spinner = create_spinner("Flattening directory...");
        
        match handle_duplicates {
            DuplicateHandling::Rename => self.flatten_with_rename(&input_dir)?,
            DuplicateHandling::Skip => self.flatten_with_skip(&input_dir)?,
        }

        spinner.finish_with_message("Directory flattening completed!");
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
        if let Some(input_dir) = &self.input_dir {
            if file.parent() == Some(input_dir.as_path()) {
                return Ok(()); // Skip files already in root
            }

            let filename = file.file_name().unwrap().to_string_lossy().to_string();
            match self.handle_duplicates {
                Some(DuplicateHandling::Rename) => {
                    let mut counter = 1;
                    let mut new_path = input_dir.join(&filename);
                    
                    while new_path.exists() {
                        let stem = file.file_stem().unwrap().to_string_lossy();
                        let ext = file.extension().map(|e| e.to_string_lossy()).unwrap_or_default();
                        let new_filename = if ext.is_empty() {
                            format!("{}-{}", stem, counter)
                        } else {
                            format!("{}-{}.{}", stem, counter, ext)
                        };
                        new_path = input_dir.join(new_filename);
                        counter += 1;
                    }
                    
                    fs::rename(file, new_path)?;
                }
                Some(DuplicateHandling::Skip) => {
                    let new_path = input_dir.join(&filename);
                    if !new_path.exists() {
                        fs::rename(file, new_path)?;
                    }
                }
                None => {}
            }
        }
        Ok(())
    }

    fn create_directories(&self, _base_dir: &PathBuf) -> Result<()> {
        // No additional directories needed for flattening
        Ok(())
    }
}

impl DirectoryFlattener {
    fn flatten_with_rename(&self, dir: &PathBuf) -> Result<()> {
        let mut filename_count: HashMap<String, u32> = HashMap::new();

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if path.parent() == Some(dir.as_path()) {
                continue; // Skip files already in root
            }

            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let count = filename_count.entry(filename.clone()).or_insert(0);
            *count += 1;

            let new_filename = if *count > 1 {
                let stem = path.file_stem().unwrap().to_string_lossy();
                let ext = path.extension().map(|e| e.to_string_lossy()).unwrap_or_default();
                if ext.is_empty() {
                    format!("{}-{}", stem, count)
                } else {
                    format!("{}-{}.{}", stem, count, ext)
                }
            } else {
                filename
            };

            let new_path = dir.join(&new_filename);
            fs::rename(path, new_path)?;
        }
        Ok(())
    }

    fn flatten_with_skip(&self, dir: &PathBuf) -> Result<()> {
        let mut existing_files: HashMap<String, bool> = HashMap::new();

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if path.parent() == Some(dir.as_path()) {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                existing_files.insert(filename, true);
                continue;
            }

            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            if !existing_files.contains_key(&filename) {
                let new_path = dir.join(&filename);
                fs::rename(path, new_path)?;
                existing_files.insert(filename, true);
            }
        }
        Ok(())
    }
} 