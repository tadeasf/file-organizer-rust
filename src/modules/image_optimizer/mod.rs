use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use image::{ImageFormat, ImageEncoder};
use rayon::prelude::*;
use std::{path::PathBuf, fs, sync::Arc, time::Duration, io::BufWriter};
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::get_directory_from_user;

pub struct ImageOptimizer {
    recursive: bool,
}

impl ImageOptimizer {
    pub fn new(recursive: bool) -> Self {
        Self { recursive }
    }

    pub async fn run(&self) -> Result<()> {
        let formats = vec!["JPEG", "PNG", "WebP"];
        let format_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select target format")
            .items(&formats)
            .default(0)
            .interact()?;

        let target_format = match format_selection {
            0 => ImageFormat::Jpeg,
            1 => ImageFormat::Png,
            2 => ImageFormat::WebP,
            _ => unreachable!(),
        };

        let input_dir = get_directory_from_user("Enter input directory path")?;
        
        // Create output directory
        let format_dir_name = match target_format {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::WebP => "webp",
            _ => unreachable!(),
        };
        let output_dir = input_dir.join(format_dir_name);
        fs::create_dir_all(&output_dir)?;

        // Collect all files first
        let files: Vec<_> = self.collect_image_files(&input_dir)?;
        let total_files = files.len();
        
        if total_files == 0 {
            println!("No image files found in the directory.");
            return Ok(());
        }

        // Create a progress bar
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg} ({pos}/{len})")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "));
        
        let pb = Arc::new(pb);
        let pb_clone = Arc::clone(&pb);

        // Start the progress bar update thread
        tokio::spawn(async move {
            loop {
                pb_clone.tick();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        // Process files in parallel with chunking for better memory management
        files.par_chunks(8)
            .try_for_each(|chunk| -> Result<()> {
                for path in chunk {
                    if let Err(e) = self.convert_image(path, &output_dir, target_format) {
                        pb.println(format!("Error converting {}: {}", path.display(), e));
                    }
                    pb.inc(1);
                    pb.set_message(format!("Converting images..."));
                }
                Ok(())
            })?;

        pb.finish_with_message(format!("Successfully converted {} images!", total_files));
        Ok(())
    }

    fn collect_image_files(&self, dir: &PathBuf) -> Result<Vec<PathBuf>> {
        let walker = if self.recursive {
            WalkDir::new(dir)
        } else {
            WalkDir::new(dir).max_depth(1)
        };

        let files: Vec<PathBuf> = walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                if let Some(ext) = e.path().extension() {
                    matches!(
                        ext.to_str().unwrap_or(""),
                        "jpg" | "jpeg" | "png" | "webp"
                    )
                } else {
                    false
                }
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        Ok(files)
    }

    fn convert_image(&self, path: &PathBuf, output_dir: &PathBuf, format: ImageFormat) -> Result<()> {
        // Open and decode the image with faster nearest-neighbor sampling
        let img = image::io::Reader::open(path)?
            .with_guessed_format()?
            .decode()?;
        
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let new_filename = format!("{}.{}", stem, format.extensions_str()[0]);
        let output_path = output_dir.join(new_filename);

        // Create a buffered writer for better performance
        let output = fs::File::create(&output_path)?;
        let mut writer = BufWriter::new(output);

        // Optimize based on format
        match format {
            ImageFormat::Jpeg => {
                // Use a lower quality setting for JPEG to improve speed
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 85);
                encoder.write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    img.color(),
                )?;
            }
            ImageFormat::Png => {
                // Use fast compression for PNG
                let encoder = image::codecs::png::PngEncoder::new_with_quality(
                    &mut writer,
                    image::codecs::png::CompressionType::Fast,
                    image::codecs::png::FilterType::Sub,
                );
                encoder.write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    img.color(),
                )?;
            }
            ImageFormat::WebP => {
                // For WebP, save directly using the image crate's save function
                // This is more reliable than using the encoder directly
                img.save_with_format(&output_path, format)?;
            }
            _ => unreachable!(),
        }
        
        Ok(())
    }
} 