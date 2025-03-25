use anyhow::Result;
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, Select, Input};
use flate2::Compression;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::utils::{create_spinner, get_directory_from_user};
use crate::modules::base::FileOrganizer;

pub struct ArchiveManager {
    recursive: bool,
    input_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    archive_type: Option<ArchiveType>,
    compression_level: Option<CompressionLevel>,
    operation_mode: Option<OperationMode>,
    split_size: Option<u64>,
}

#[derive(Clone, Copy)]
enum ArchiveType {
    Zip,
    Tar,
    TarGz,
    TarZst,
}

#[derive(Clone, Copy)]
enum CompressionLevel {
    None,
    Fast,
    Balanced,
    Best,
}

#[derive(Clone, Copy)]
enum OperationMode {
    Create,
    Extract,
    Update,
    Split,
}

impl ArchiveType {
    fn extension(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::TarZst => "tar.zst",
        }
    }
}

#[async_trait]
impl FileOrganizer for ArchiveManager {
    fn new(recursive: bool) -> Self {
        Self {
            recursive,
            input_dir: None,
            output_dir: None,
            archive_type: None,
            compression_level: None,
            operation_mode: None,
            split_size: None,
        }
    }

    async fn run(&self) -> Result<()> {
        let operation_options = vec!["Create Archive", "Extract Archive", "Update Archive", "Split Archive"];
        let operation_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select operation")
            .items(&operation_options)
            .default(0)
            .interact()?;

        let operation_mode = match operation_selection {
            0 => OperationMode::Create,
            1 => OperationMode::Extract,
            2 => OperationMode::Update,
            3 => OperationMode::Split,
            _ => unreachable!(),
        };

        let archive_options = vec!["ZIP", "TAR", "TAR.GZ", "TAR.ZST"];
        let archive_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select archive format")
            .items(&archive_options)
            .default(0)
            .interact()?;

        let archive_type = match archive_selection {
            0 => ArchiveType::Zip,
            1 => ArchiveType::Tar,
            2 => ArchiveType::TarGz,
            3 => ArchiveType::TarZst,
            _ => unreachable!(),
        };

        let compression_options = vec!["None", "Fast", "Balanced", "Best"];
        let compression_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select compression level")
            .items(&compression_options)
            .default(2)
            .interact()?;

        let compression_level = match compression_selection {
            0 => CompressionLevel::None,
            1 => CompressionLevel::Fast,
            2 => CompressionLevel::Balanced,
            3 => CompressionLevel::Best,
            _ => unreachable!(),
        };

        let input_dir = get_directory_from_user("Enter input directory path")?;
        let output_dir = if matches!(operation_mode, OperationMode::Extract) {
            input_dir.clone()
        } else {
            input_dir.parent().unwrap_or(&input_dir).to_path_buf()
        };

        let split_size = if matches!(operation_mode, OperationMode::Split) {
            let size_str: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter split size (e.g., 100MB, 1GB)")
                .interact_text()?;
            Some(parse_size(&size_str)?)
        } else {
            None
        };

        let mut this = Self {
            recursive: self.recursive,
            input_dir: Some(input_dir.clone()),
            output_dir: Some(output_dir),
            archive_type: Some(archive_type),
            compression_level: Some(compression_level),
            operation_mode: Some(operation_mode),
            split_size,
        };

        let spinner = create_spinner("Processing archive...");

        match operation_mode {
            OperationMode::Create => this.create_archive()?,
            OperationMode::Extract => this.extract_archive()?,
            OperationMode::Update => this.update_archive()?,
            OperationMode::Split => this.split_archive()?,
        }

        spinner.finish_with_message("Archive operation completed successfully!");
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
        match self.operation_mode.unwrap() {
            OperationMode::Create | OperationMode::Update => {
                let input_dir = self.input_dir.as_ref().unwrap();
                let relative_path = file.strip_prefix(input_dir)?;
                let target_path = self.output_dir.as_ref().unwrap().join(relative_path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(file, target_path)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn create_directories(&self, base_dir: &PathBuf) -> Result<()> {
        match self.operation_mode.unwrap() {
            OperationMode::Create | OperationMode::Update => {
                if let Some(output_dir) = &self.output_dir {
                    fs::create_dir_all(output_dir.join(base_dir))?;
                }
            }
            OperationMode::Extract => {
                fs::create_dir_all(base_dir)?;
            }
            OperationMode::Split => {
                if let Some(output_dir) = &self.output_dir {
                    fs::create_dir_all(output_dir)?;
                }
            }
        }
        Ok(())
    }
}

impl ArchiveManager {
    fn create_archive(&self) -> Result<()> {
        if !matches!(self.operation_mode.unwrap(), OperationMode::Create) {
            anyhow::bail!("Invalid operation mode for create_archive");
        }

        let input_dir = self.input_dir.as_ref().unwrap();
        let output_dir = self.output_dir.as_ref().unwrap();
        let archive_name = format!(
            "{}.{}",
            input_dir.file_name().unwrap().to_string_lossy(),
            self.archive_type.unwrap().extension()
        );
        let archive_path = output_dir.join(archive_name);

        match self.archive_type.unwrap() {
            ArchiveType::Zip => self.create_zip_archive(&archive_path)?,
            ArchiveType::Tar => self.create_tar_archive(&archive_path, None)?,
            ArchiveType::TarGz => self.create_tar_archive(&archive_path, Some(Compression::default()))?,
            ArchiveType::TarZst => self.create_zst_archive(&archive_path)?,
        }

        Ok(())
    }

    fn create_zip_archive(&self, archive_path: &PathBuf) -> Result<()> {
        let file = File::create(archive_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(match self.compression_level.unwrap() {
                CompressionLevel::None => CompressionMethod::Stored,
                _ => CompressionMethod::Deflated,
            })
            .unix_permissions(0o755);

        let input_dir = self.input_dir.as_ref().unwrap();
        let base_path = input_dir.as_path();

        for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.strip_prefix(base_path)?;

            if path.is_file() {
                zip.start_file(name.to_string_lossy(), options)?;
                let mut f = File::open(path)?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    fn create_tar_archive(&self, archive_path: &PathBuf, compression: Option<Compression>) -> Result<()> {
        let file = File::create(archive_path)?;
        let writer: Box<dyn Write> = if let Some(level) = compression {
            Box::new(flate2::write::GzEncoder::new(file, level))
        } else {
            Box::new(file)
        };
        let mut builder = tar::Builder::new(writer);

        let input_dir = self.input_dir.as_ref().unwrap();
        let base_path = input_dir.as_path();

        for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let name = path.strip_prefix(base_path)?;
                builder.append_path_with_name(path, name)?;
            }
        }

        builder.finish()?;
        Ok(())
    }

    fn create_zst_archive(&self, archive_path: &PathBuf) -> Result<()> {
        let file = File::create(archive_path)?;
        let level = match self.compression_level.unwrap() {
            CompressionLevel::None => 1,
            CompressionLevel::Fast => 3,
            CompressionLevel::Balanced => 10,
            CompressionLevel::Best => 19,
        };
        
        let encoder = zstd::Encoder::new(file, level)?;
        let mut builder = tar::Builder::new(encoder);

        let input_dir = self.input_dir.as_ref().unwrap();
        let base_path = input_dir.as_path();

        for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let name = path.strip_prefix(base_path)?;
                builder.append_path_with_name(path, name)?;
            }
        }

        let encoder = builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    fn extract_archive(&self) -> Result<()> {
        if !matches!(self.operation_mode.unwrap(), OperationMode::Extract) {
            anyhow::bail!("Invalid operation mode for extract_archive");
        }

        let input_dir = self.input_dir.as_ref().unwrap();
        let output_dir = self.output_dir.as_ref().unwrap();

        match self.archive_type.unwrap() {
            ArchiveType::Zip => self.extract_zip_archive(input_dir, output_dir)?,
            ArchiveType::Tar => self.extract_tar_archive(input_dir, output_dir, None)?,
            ArchiveType::TarGz => self.extract_tar_archive(input_dir, output_dir, Some("gz"))?,
            ArchiveType::TarZst => self.extract_tar_archive(input_dir, output_dir, Some("zst"))?,
        }

        Ok(())
    }

    fn extract_zip_archive(&self, archive_path: &PathBuf, output_dir: &PathBuf) -> Result<()> {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut outfile = File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    fn extract_tar_archive(&self, archive_path: &PathBuf, output_dir: &PathBuf, compression: Option<&str>) -> Result<()> {
        let file = File::open(archive_path)?;
        let reader: Box<dyn Read> = match compression {
            Some("gz") => Box::new(flate2::read::GzDecoder::new(file)),
            Some("zst") => Box::new(zstd::Decoder::new(file)?),
            _ => Box::new(file),
        };

        let mut archive = tar::Archive::new(reader);
        archive.unpack(output_dir)?;

        Ok(())
    }

    fn update_archive(&mut self) -> Result<()> {
        if !matches!(self.operation_mode.unwrap(), OperationMode::Update) {
            anyhow::bail!("Invalid operation mode for update_archive");
        }

        let temp_dir = self.output_dir.as_ref().unwrap().join("temp_extract");
        fs::create_dir_all(&temp_dir)?;

        let mut temp_manager = Self::new(true);
        temp_manager.input_dir = self.input_dir.clone();
        temp_manager.output_dir = Some(temp_dir.clone());
        temp_manager.archive_type = self.archive_type;
        temp_manager.extract_archive()?;

        let input_dir = self.input_dir.as_ref().unwrap();
        for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let relative_path = path.strip_prefix(input_dir)?;
                let target_path = temp_dir.join(relative_path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, target_path)?;
            }
        }

        self.input_dir = Some(temp_dir.clone());
        self.create_archive()?;

        fs::remove_dir_all(temp_dir)?;
        Ok(())
    }

    fn split_archive(&self) -> Result<()> {
        if !matches!(self.operation_mode.unwrap(), OperationMode::Split) {
            anyhow::bail!("Invalid operation mode for split_archive");
        }

        let input_dir = self.input_dir.as_ref().unwrap();
        let output_dir = self.output_dir.as_ref().unwrap();
        let split_size = self.split_size.unwrap();

        let mut current_size = 0;
        let mut current_part = 1;
        let mut current_archive = None;

        for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_size = fs::metadata(path)?.len();
            if current_size + file_size > split_size || current_archive.is_none() {
                let archive_name = format!(
                    "{}.part{}.{}",
                    input_dir.file_name().unwrap().to_string_lossy(),
                    current_part,
                    self.archive_type.unwrap().extension()
                );
                let archive_path = output_dir.join(archive_name);

                match self.archive_type.unwrap() {
                    ArchiveType::Zip => {
                        let file = File::create(&archive_path)?;
                        current_archive = Some(ZipWriter::new(file));
                    }
                    _ => anyhow::bail!("Split operation is currently only supported for ZIP archives"),
                }

                current_size = 0;
                current_part += 1;
            }

            if let Some(archive) = current_archive.as_mut() {
                let name = path.strip_prefix(input_dir)?.to_string_lossy();
                let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
                archive.start_file(name.to_string(), options)?;
                
                let mut f = File::open(path)?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)?;
                archive.write_all(&buffer)?;
                
                current_size += file_size;
            }
        }

        if let Some(mut archive) = current_archive {
            archive.finish()?;
        }

        Ok(())
    }
}

fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim().to_lowercase();
    let mut num = String::new();
    let mut unit = String::new();

    for c in size_str.chars() {
        if c.is_digit(10) || c == '.' {
            num.push(c);
        } else {
            unit.push(c);
        }
    }

    let number: f64 = num.parse()?;
    let multiplier = match unit.as_str() {
        "b" => 1,
        "kb" => 1024,
        "mb" => 1024 * 1024,
        "gb" => 1024 * 1024 * 1024,
        _ => anyhow::bail!("Invalid size unit. Use B, KB, MB, or GB"),
    };

    Ok((number * multiplier as f64) as u64)
} 