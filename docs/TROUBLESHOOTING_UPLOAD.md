# Troubleshooting Dataset Upload Errors

## Error: "No such file or directory (os error 2)"

This error means the application cannot find the file at the path you provided.

### ✅ Solutions

#### 1. **Use Absolute Paths**
Provide the complete path to your file:
```
/home/username/Documents/my_data.csv
```

#### 2. **Use Home Directory Shortcut**
The `~` symbol now expands to your home directory:
```
~/Documents/my_data.csv
~/Desktop/experiment.tsv
```

#### 3. **Use Relative Paths**
If your file is in the same directory where you ran the app:
```
./my_data.csv
./data/samples.tsv
```

#### 4. **Drag and Drop**
In most terminals, you can drag a file from your file manager directly into the terminal window, and it will paste the full path.

#### 5. **Use Tab Completion**
Start typing the path and press Tab to auto-complete:
```
~/Doc[Tab] → ~/Documents/
```

### 🔍 Checking Your File Path

Before pasting the path into the app, verify it exists:

```bash
# Check if file exists
ls -l /path/to/your/file.csv

# Or use the 'file' command
file /path/to/your/file.csv
```

### 📋 Path Examples

**Absolute paths (recommended):**
- `/home/john/data/microarray_2024.csv`
- `/mnt/storage/research/gene_expression.tsv`

**Home directory paths:**
- `~/experiments/aging_study.csv`
- `~/Downloads/dataset.tsv`

**Relative paths:**
- `./data.csv` (current directory)
- `../parent_folder/data.csv` (parent directory)
- `data/experiment1/results.csv` (subdirectory)

### ⚠️ Common Issues

#### Spaces in Path
If your path contains spaces, the terminal might split it. Make sure to paste it as-is:
```
❌ /home/user/My Documents/data.csv (might be split)
✅ /home/user/My\ Documents/data.csv (escaped)
✅ "/home/user/My Documents/data.csv" (quoted)
```

The app now handles this automatically!

#### Hidden Files/Directories
Files or directories starting with `.` are hidden:
```
~/.hidden_folder/data.csv
```

#### Wrong File Extension
Only `.csv` and `.tsv` files are supported:
```
❌ data.xlsx (not supported)
❌ data.txt (not supported)
✅ data.csv (supported)
✅ data.tsv (supported)
```

### 📊 File Requirements

Your dataset file must:

1. **Be in CSV or TSV format**
   - Comma-separated (`.csv`) or tab-separated (`.tsv`)

2. **Have headers in the first row**
   - Column names should be in the first line

3. **Include required columns:**
   - **Ensembl ID column**: Can be named any of:
     - `ensembl_id`
     - `Ensembl ID`
     - `ensembl`
     - Or any variation containing "ensembl"
   
   - **Age column**: Can be named any of:
     - `age`
     - `Age`
     - `AGE`
     - Or any variation containing "age"

### 📝 Example Dataset Structure

```csv
ensembl_id,age,cell_type,expression_level
ENSG00000000003,25,T-cell,5.23
ENSG00000000005,30,B-cell,6.45
ENSG00000000419,35,T-cell,4.89
```

### 🆘 Still Having Issues?

If the error persists:

1. **Check file permissions:**
   ```bash
   ls -l /path/to/your/file.csv
   ```
   Make sure you have read permissions (should show `r` in permissions)

2. **Verify the file exists:**
   ```bash
   test -f /path/to/your/file.csv && echo "File exists" || echo "File not found"
   ```

3. **Check the file content:**
   ```bash
   head -n 5 /path/to/your/file.csv
   ```

4. **Copy the file to a simpler path:**
   ```bash
   cp /complicated/path/to/file.csv ~/data.csv
   ```
   Then use: `~/data.csv`

### 🎯 Enhanced Error Messages

The application now provides better error messages:

- ✅ Shows the exact path it tried to access
- ✅ Suggests common fixes
- ✅ Automatically expands `~` to your home directory
- ✅ Converts relative paths to absolute paths
- ✅ Checks if file exists before trying to read it
- ✅ Validates file extension before processing

### 💡 Quick Test

To test with a sample file:

```bash
# Create a test CSV file
cat > ~/test_data.csv << EOF
ensembl_id,age,expression
ENSG00000000003,25,5.23
ENSG00000000005,30,6.45
EOF

# Now paste this path in the app:
~/test_data.csv
```
