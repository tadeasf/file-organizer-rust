use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use std::{collections::HashMap, fs, path::PathBuf};
use walkdir::WalkDir;

use crate::utils::{create_spinner, get_directory_from_user};

pub struct DirectoryFlattener;

impl DirectoryFlattener {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self) -> Result<()> {
        let input_dir = get_directory_from_user("Enter directory to flatten")?;
        
        let options = vec!["Rename duplicates", "Skip duplicates"];
        let handle_duplicates = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How to handle duplicate filenames?")
            .items(&options)
            .default(0)
            .interact()?;

        let spinner = create_spinner("Flattening directory...");
        
        match handle_duplicates {
            0 => self.flatten_with_rename(&input_dir)?,
            1 => self.flatten_with_skip(&input_dir)?,
            _ => unreachable!(),
        }

        spinner.finish_with_message("Directory flattening completed!");
        Ok(())
    }

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