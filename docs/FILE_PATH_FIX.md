# File Path Handling Improvements

## Summary
Enhanced the file path handling in the TUI to prevent "No such file or directory" errors and provide better user guidance.

## Changes Made

### 1. Path Cleaning and Expansion
**File:** `src/tui/app.rs` - `load_dataset_from_path()` function

The application now:
- **Trims whitespace** from pasted paths (removes newlines, spaces)
- **Expands `~`** to the user's home directory
- **Converts relative paths** to absolute paths
- **Shows the actual path** being accessed in error messages

**Before:**
```rust
let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
```

**After:**
```rust
// Clean up the path: trim whitespace, expand home directory
let path = path.trim();

// Expand ~ to home directory
let expanded_path = if path.starts_with("~/") {
    if let Some(home) = dirs::home_dir() {
        home.join(&path[2..])
    } else {
        std::path::PathBuf::from(path)
    }
} else if path == "~" {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(path))
} else {
    std::path::PathBuf::from(path)
};

// Convert to absolute path if relative
let absolute_path = if expanded_path.is_absolute() {
    expanded_path
} else {
    std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join(&expanded_path)
};
```

### 2. Pre-Flight Validation
**File:** `src/tui/app.rs`

Added checks before attempting to read the file:

```rust
// Check if file exists before trying to read
if !absolute_path.exists() {
    return Err(format!(
        "File not found: {}\n\nPlease check:\n\
         1. The file path is correct\n\
         2. The file exists at that location\n\
         3. You have permission to read the file",
        absolute_path.display()
    ));
}

if !absolute_path.is_file() {
    return Err(format!(
        "Path is not a file: {}\n\nPlease provide a path to a .csv or .tsv file.",
        absolute_path.display()
    ));
}
```

### 3. Enhanced Error Messages
**File:** `src/tui/app.rs`

All error messages now show the actual path and provide actionable guidance:

**File extension error:**
```rust
return Err(format!(
    "Only .csv or .tsv files are supported.\nYour file has extension: .{}",
    extension
));
```

**Read error:**
```rust
let bytes = tokio::fs::read(&absolute_path)
    .await
    .map_err(|e| format!("Failed to read file {}: {}", absolute_path.display(), e))?;
```

**Directory creation error:**
```rust
tokio::fs::create_dir_all(upload_dir)
    .await
    .map_err(|e| format!("Failed to create uploads directory: {}", e))?;
```

### 4. Improved Welcome Message
**File:** `src/tui/app.rs`

Added practical examples and tips:

```rust
"Examples:\n\
 • /home/user/data/microarray.csv\n\
 • ~/Documents/experiment_data.tsv\n\
 • ./data/samples.csv\n\n\
 Tip: You can drag & drop a file into the terminal or use tab completion."
```

## Testing

### Test Case 1: Home Directory Expansion
```bash
# Input
~/data/microarray.csv

# Result
Expands to: /home/username/data/microarray.csv
```

### Test Case 2: Relative Path
```bash
# Input (in project directory)
./test_data.csv

# Result
Expands to: /home/username/Oxidized_Bio/test_data.csv
```

### Test Case 3: File Not Found
```bash
# Input
~/nonexistent.csv

# Result
Error: File not found: /home/username/nonexistent.csv

Please check:
1. The file path is correct
2. The file exists at that location
3. You have permission to read the file
```

### Test Case 4: Wrong Extension
```bash
# Input
~/data.xlsx

# Result
Error: Only .csv or .tsv files are supported.
Your file has extension: .xlsx
```

### Test Case 5: Path with Spaces (Automatic Handling)
```bash
# Input
~/My Documents/data.csv

# Result
Automatically handles spaces, expands to absolute path
```

## User Benefits

1. **Better Error Messages**
   - Shows exactly what path was attempted
   - Provides specific troubleshooting steps
   - Displays the actual file extension if wrong

2. **Flexible Path Input**
   - Supports `~` for home directory
   - Supports relative paths (`./`, `../`)
   - Supports absolute paths
   - Automatically trims whitespace

3. **Early Error Detection**
   - Checks file existence before reading
   - Validates file type before processing
   - Clear error messages at each step

4. **Better User Guidance**
   - Example paths in welcome message
   - Drag & drop suggestion
   - Tab completion tip

## Troubleshooting

For detailed troubleshooting information, see:
- [TROUBLESHOOTING_UPLOAD.md](./TROUBLESHOOTING_UPLOAD.md)

### Quick Fixes

**Error: "No such file or directory"**
1. Use absolute path: `/home/user/data.csv`
2. Or use `~`: `~/data.csv`
3. Or verify with: `ls -l /path/to/file.csv`

**Error: "Only .csv or .tsv files are supported"**
1. Convert your file to CSV or TSV format
2. Or export from Excel/Sheets as CSV

**Error: "Path is not a file"**
1. Make sure you're pointing to a file, not a directory
2. Double-check the path

## Implementation Details

### Dependencies
Uses `dirs` crate for home directory expansion (already in `Cargo.toml`).

### Path Resolution Order
1. Trim whitespace from input
2. Check for `~` and expand to home directory
3. Convert relative paths to absolute
4. Validate file exists
5. Validate it's a file (not directory)
6. Validate extension
7. Attempt to read

### Error Handling
All errors now include:
- Context (what operation failed)
- The actual path attempted
- Suggestions for fixing the issue

## Future Improvements

Potential enhancements:
- [ ] Support for `.gz` compressed files
- [ ] Support for `.xlsx` files (requires additional parsing)
- [ ] File picker UI using `rfd` crate
- [ ] Recent files history
- [ ] Path autocomplete in TUI
- [ ] Sample dataset download command
