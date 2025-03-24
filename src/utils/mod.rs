use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb
}

pub fn get_directory_from_user(prompt: &str) -> Result<PathBuf> {
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()?;
    
    let path = PathBuf::from(path);
    if !path.exists() {
        anyhow::bail!("Directory does not exist");
    }
    if !path.is_dir() {
        anyhow::bail!("Path is not a directory");
    }
    Ok(path)
} 