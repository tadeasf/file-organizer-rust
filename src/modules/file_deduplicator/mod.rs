use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select, Confirm};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::HashMap,
    fs,
    io::{Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use rayon::prelude::*;

use crate::utils::get_directory_from_user;

pub struct FileDeduplicator {
    recursive: bool,
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    hash: Option<String>,
}

impl FileDeduplicator {
    pub fn new(recursive: bool) -> Self {
        Self { recursive }
    }

    pub async fn run(&self) -> Result<()> {
        let input_dir = get_directory_from_user("Enter directory to scan for duplicates")?;

        // Choose comparison method
        let methods = vec!["Hash (most accurate)", "Size only (fastest)", "Name only"];
        let method = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select comparison method")
            .items(&methods)
            .default(0)
            .interact()?;

        // Choose action for duplicates
        let actions = vec![
            "Generate report only",
            "Move duplicates to separate directory",
            "Delete duplicates (careful!)",
        ];
        let action = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What to do with duplicates?")
            .items(&actions)
            .default(0)
            .interact()?;

        // Collect and process files
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.set_message("Scanning files...");

        let mut files = self.collect_files(&input_dir)?;
        spinner.set_message(format!("Found {} files. Processing...", files.len()));

        let duplicates = match method {
            0 => self.find_duplicates_by_hash(&mut files, &spinner)?,
            1 => self.find_duplicates_by_size(&files)?,
            2 => self.find_duplicates_by_name(&files)?,
            _ => unreachable!(),
        };

        if duplicates.is_empty() {
            spinner.finish_with_message("No duplicates found!");
            return Ok(());
        }

        spinner.finish_with_message(format!("Found {} groups of duplicates!", duplicates.len()));

        match action {
            0 => self.generate_report(&duplicates)?,
            1 => self.move_duplicates(&duplicates)?,
            2 => self.delete_duplicates(&duplicates)?,
            _ => unreachable!(),
        }

        Ok(())
    }

    fn collect_files(&self, dir: &Path) -> Result<Vec<FileInfo>> {
        let walker = if self.recursive {
            WalkDir::new(dir)
        } else {
            WalkDir::new(dir).max_depth(1)
        };

        let files: Vec<FileInfo> = walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let metadata = e.metadata().unwrap();
                FileInfo {
                    path: e.path().to_path_buf(),
                    size: metadata.len(),
                    hash: None,
                }
            })
            .collect();

        Ok(files)
    }

    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024 * 1024]; // 1MB buffer

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn find_duplicates_by_hash(
        &self,
        files: &mut Vec<FileInfo>,
        spinner: &ProgressBar,
    ) -> Result<Vec<Vec<PathBuf>>> {
        let pb = Arc::new(spinner.clone());
        
        // First group by size to reduce hash calculations
        let size_groups: HashMap<u64, Vec<&mut FileInfo>> = files
            .iter_mut()
            .filter(|f| f.size > 0) // Skip empty files
            .fold(HashMap::new(), |mut acc, file| {
                acc.entry(file.size).or_default().push(file);
                acc
            });

        // Only calculate hashes for files with same size
        let potential_duplicates: Vec<&mut FileInfo> = size_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .flat_map(|(_, group)| group)
            .collect();

        // Calculate hashes in parallel
        potential_duplicates.into_par_iter().for_each(|file| {
            if let Ok(hash) = self.calculate_file_hash(&file.path) {
                file.hash = Some(hash);
            }
            pb.inc(1);
        });

        // Group by hash
        let mut hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for file in files.iter().filter(|f| f.hash.is_some()) {
            if let Some(hash) = &file.hash {
                hash_groups.entry(hash.clone())
                    .or_default()
                    .push(file.path.clone());
            }
        }

        // Convert to vector of duplicate groups
        Ok(hash_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, group)| group)
            .collect())
    }

    fn find_duplicates_by_size(&self, files: &[FileInfo]) -> Result<Vec<Vec<PathBuf>>> {
        let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        
        for file in files {
            size_groups.entry(file.size)
                .or_default()
                .push(file.path.clone());
        }

        Ok(size_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, group)| group)
            .collect())
    }

    fn find_duplicates_by_name(&self, files: &[FileInfo]) -> Result<Vec<Vec<PathBuf>>> {
        let mut name_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
        
        for file in files {
            if let Some(name) = file.path.file_name() {
                if let Some(name_str) = name.to_str() {
                    name_groups.entry(name_str.to_string())
                        .or_default()
                        .push(file.path.clone());
                }
            }
        }

        Ok(name_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, group)| group)
            .collect())
    }

    fn generate_report(&self, duplicates: &[Vec<PathBuf>]) -> Result<()> {
        let report_path = "duplicates_report.txt";
        let mut report = String::new();
        
        report.push_str("=== Duplicate Files Report ===\n\n");
        
        for (i, group) in duplicates.iter().enumerate() {
            report.push_str(&format!("Group {}:\n", i + 1));
            for path in group {
                report.push_str(&format!("  {}\n", path.display()));
            }
            report.push('\n');
        }

        fs::write(report_path, report)?;
        println!("Report generated: {}", report_path);
        Ok(())
    }

    fn move_duplicates(&self, duplicates: &[Vec<PathBuf>]) -> Result<()> {
        let duplicates_dir = PathBuf::from("duplicates");
        fs::create_dir_all(&duplicates_dir)?;

        for group in duplicates {
            // Keep the first file as original
            for path in group.iter().skip(1) {
                let new_path = duplicates_dir.join(path.file_name().unwrap());
                fs::rename(path, new_path)?;
            }
        }

        println!("Duplicates moved to: {}", duplicates_dir.display());
        Ok(())
    }

    fn delete_duplicates(&self, duplicates: &[Vec<PathBuf>]) -> Result<()> {
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Are you sure you want to delete all duplicate files? This cannot be undone!")
            .default(false)
            .interact()?;

        if !confirm {
            println!("Operation cancelled");
            return Ok(());
        }

        let mut deleted_count = 0;
        for group in duplicates {
            // Keep the first file as original
            for path in group.iter().skip(1) {
                fs::remove_file(path)?;
                deleted_count += 1;
            }
        }

        println!("Deleted {} duplicate files", deleted_count);
        Ok(())
    }
} 