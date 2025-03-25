use anyhow::Result;
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use std::{collections::HashMap, fs, path::PathBuf};
use walkdir::WalkDir;

use crate::utils::{create_spinner, get_directory_from_user};
use crate::modules::base::FileOrganizer;

pub struct FileCategorizer {
    recursive: bool,
    input_dir: Option<PathBuf>,
    rules: Vec<CategoryRule>,
}

#[derive(Debug, Clone)]
enum CategoryRule {
    FileType,
    DateBased,
    Custom(HashMap<String, Vec<String>>),
}

#[async_trait]
impl FileOrganizer for FileCategorizer {
    fn new(recursive: bool) -> Self {
        Self {
            recursive,
            input_dir: None,
            rules: Vec::new(),
        }
    }

    async fn run(&self) -> Result<()> {
        let input_dir = get_directory_from_user("Enter directory to categorize")?;
        
        let rule_options = vec!["File Type", "Date Based", "Custom Rules"];
        let selected_rules = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select categorization rules")
            .items(&rule_options)
            .defaults(&[true, false, false])
            .interact()?;

        if selected_rules.is_empty() {
            anyhow::bail!("No categorization rules selected");
        }

        let mut rules = Vec::new();
        for &idx in selected_rules.iter() {
            match idx {
                0 => rules.push(CategoryRule::FileType),
                1 => rules.push(CategoryRule::DateBased),
                2 => {
                    let custom_rules = self.configure_custom_rules()?;
                    rules.push(CategoryRule::Custom(custom_rules));
                }
                _ => unreachable!(),
            }
        }

        let spinner = create_spinner("Categorizing files...");
        self.categorize_files(&input_dir, &rules)?;
        spinner.finish_with_message("File categorization completed!");

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
            for rule in &self.rules {
                match rule {
                    CategoryRule::FileType => self.categorize_by_type(file, input_dir)?,
                    CategoryRule::DateBased => self.categorize_by_date(file, input_dir)?,
                    CategoryRule::Custom(rules) => self.categorize_by_custom_rules(file, input_dir, rules)?,
                }
            }
        }
        Ok(())
    }

    fn create_directories(&self, base_dir: &PathBuf) -> Result<()> {
        for rule in &self.rules {
            match rule {
                CategoryRule::FileType => {
                    fs::create_dir_all(base_dir.join("by_type"))?;
                }
                CategoryRule::DateBased => {
                    fs::create_dir_all(base_dir.join("by_date"))?;
                }
                CategoryRule::Custom(rules) => {
                    for category in rules.keys() {
                        fs::create_dir_all(base_dir.join("custom").join(category))?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl FileCategorizer {
    fn configure_custom_rules(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut rules = HashMap::new();
        println!("Configure custom rules (category:extension, e.g., 'Documents:pdf,doc,docx')");
        println!("Enter an empty line to finish");

        loop {
            let input: String = dialoguer::Input::new()
                .with_prompt("Enter rule")
                .allow_empty(true)
                .interact_text()?;

            if input.is_empty() {
                break;
            }

            let parts: Vec<&str> = input.split(':').collect();
            if parts.len() != 2 {
                println!("Invalid format. Use 'Category:ext1,ext2,...'");
                continue;
            }

            let category = parts[0].trim().to_string();
            let extensions: Vec<String> = parts[1]
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect();

            rules.insert(category, extensions);
        }

        Ok(rules)
    }

    fn categorize_files(&self, dir: &PathBuf, _rules: &[CategoryRule]) -> Result<()> {
        let walker = if self.recursive {
            WalkDir::new(dir)
        } else {
            WalkDir::new(dir).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path().to_path_buf();
            self.process_file(&path)?;
        }

        Ok(())
    }

    fn categorize_by_type(&self, file: &PathBuf, base_dir: &PathBuf) -> Result<()> {
        if let Some(ext) = file.extension() {
            let category = ext.to_string_lossy().to_lowercase();
            let target_dir = base_dir.join("by_type").join(&category);
            fs::create_dir_all(&target_dir)?;
            
            let target_path = target_dir.join(file.file_name().unwrap());
            if !target_path.exists() {
                fs::rename(file, target_path)?;
            }
        }
        Ok(())
    }

    fn categorize_by_date(&self, file: &PathBuf, base_dir: &PathBuf) -> Result<()> {
        let metadata = fs::metadata(file)?;
        let created = metadata.created()?;
        let datetime = chrono::DateTime::<chrono::Local>::from(created);
        
        let year = datetime.format("%Y").to_string();
        let month = datetime.format("%m-%B").to_string();
        
        let target_dir = base_dir.join("by_date").join(&year).join(&month);
        fs::create_dir_all(&target_dir)?;
        
        let target_path = target_dir.join(file.file_name().unwrap());
        if !target_path.exists() {
            fs::rename(file, target_path)?;
        }
        Ok(())
    }

    fn categorize_by_custom_rules(
        &self,
        file: &PathBuf,
        base_dir: &PathBuf,
        rules: &HashMap<String, Vec<String>>,
    ) -> Result<()> {
        if let Some(ext) = file.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            
            for (category, extensions) in rules {
                if extensions.contains(&ext) {
                    let target_dir = base_dir.join("custom").join(category);
                    fs::create_dir_all(&target_dir)?;
                    
                    let target_path = target_dir.join(file.file_name().unwrap());
                    if !target_path.exists() {
                        fs::rename(file, target_path)?;
                    }
                    break;
                }
            }
        }
        Ok(())
    }
} 