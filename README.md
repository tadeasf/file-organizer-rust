# File Organizer (Rust)

A modular command-line tool built in Rust for efficient file and image handling operations. The tool is designed with extensibility in mind, making it easy to add new modules for different file management tasks.

## Features

Currently implemented modules:

### Image Optimizer
- Converts images between different formats (JPEG, PNG, WebP)
- Supports recursive directory processing
- Parallel processing for better performance
- Progress tracking with interactive display
- Creates format-specific output directories
- Optimized encoding settings for each format

### Directory Flattener
- Flattens nested directory structures into a single directory
- Two modes for handling duplicate files:
  - Rename duplicates (adds numerical suffix)
  - Skip duplicates (keeps first occurrence)
- Interactive progress display

### File Deduplicator
- Finds duplicate files using various hash methods
- Supports multiple duplicate handling strategies
- Generates detailed reports
- Supports recursive operation

### Archive Manager
- Supports multiple archive formats (ZIP, TAR, TAR.GZ, TAR.ZST)
- Multiple operation modes:
  - Create: Create new archives with customizable compression
  - Extract: Extract archives to specified locations
  - Update: Add or update files in existing archives
  - Split: Split large archives into smaller parts (ZIP only)
- Compression options:
  - None: No compression
  - Fast: Quick compression
  - Balanced: Default compression
  - Best: Maximum compression
- Progress tracking and user feedback
- Supports recursive operation

## Usage

Run the tool without arguments for an interactive menu, or use command-line arguments:

```bash
# Interactive mode
./file-organizer-rust

# Direct command usage
./file-organizer-rust image-optimize --recursive  # Optimize images recursively
./file-organizer-rust directory-flatten           # Flatten a directory
./file-organizer-rust deduplicate --recursive       # Find and handle duplicates
./file-organizer-rust archive --recursive           # Manage archives
```

## Project Structure

```
src/
├── cli/         # Command-line interface handling
├── modules/     # Individual feature modules
│   ├── directory_flattener/
│   └── image_optimizer/
└── utils/       # Shared utility functions
```

## Extensibility

The project is designed to be modular and extensible. To add a new module:

1. Create a new directory under `src/modules/`
2. Implement your module's functionality
3. Add the module to `src/modules/mod.rs`
4. Register the module in `src/cli/mod.rs`

## Contributing

We welcome new modules and improvements! Here are the modules we're planning to implement next:

### 1. File Deduplicator
- Scan directories for duplicate files using hash algorithms (MD5, SHA256)
- Identify and handle duplicate files with options to:
  - Remove duplicates automatically
  - Move duplicates to a separate directory
  - Generate a report of duplicates
- Support for different comparison methods (hash, name, size)

### 2. Batch Renaming Tool
- Rename multiple files using customizable patterns
- Features planned:
  - Add prefixes/suffixes
  - Sequential numbering
  - Date-based naming
  - Regex-based replacements
  - Case conversion
  - Character substitution

### 3. File Categorizer
- Automatically organize files into logical directory structures
- Categorization methods:
  - File type/extension
  - Creation/modification date
  - Content analysis
  - Custom rules and filters
- Support for custom category definitions

### 4. Metadata Editor
- View and edit file metadata
- Support for:
  - EXIF data for images
  - Audio file tags
  - Video metadata
  - Document properties
- Batch metadata operations

### 5. Archive Manager & Compression Optimizer
- Comprehensive archive handling:
  - Create/extract archives (ZIP, TAR, RAR)
  - Update existing archives
  - Split large archives
- Compression optimization:
  - Smart compression selection
  - File-type specific optimization
  - Batch compression tasks

### 6. Thumbnail Generator
- Create thumbnails for visual file preview
- Features planned:
  - Multiple thumbnail sizes
  - Custom output formats
  - Batch processing
  - Support for images and videos
  - Gallery generation

## Dependencies

- tokio - Async runtime
- image - Image processing
- walkdir - Directory traversal
- rayon - Parallel processing
- dialoguer - Interactive CLI
- indicatif - Progress bars
- anyhow - Error handling
- clap - Command line argument parsing

## License

MIT License
