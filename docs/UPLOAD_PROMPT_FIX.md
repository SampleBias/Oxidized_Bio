# Data Upload Prompt Enhancement

## Summary
Enhanced the TUI to make the dataset upload prompt more prominent and clear when the application starts in automated workflow mode.

## Changes Made

### 1. Updated Input Placeholder Text
**File:** `src/tui/app.rs`

**Before:**
```rust
input.set_placeholder_text("Type your research question here...");
```

**After:**
```rust
input.set_placeholder_text("Paste dataset path (CSV/TSV with Ensembl ID + Age columns)...");
```

This change makes it immediately clear that the application expects a dataset path as the first input.

### 2. Dynamic Placeholder Based on Workflow Stage
**File:** `src/tui/app.rs` (in `submit_message()` function)

**Before:**
```rust
self.input = TextArea::default();
self.input.set_placeholder_text("Type a question or /help for commands...");
```

**After:**
```rust
self.input = TextArea::default();
// Update placeholder based on workflow stage
let placeholder = match self.workflow_stage {
    WorkflowStage::Upload => "Paste dataset path (CSV/TSV with Ensembl ID + Age columns)...",
    _ => "Type a question or /help for commands...",
};
self.input.set_placeholder_text(placeholder);
```

This ensures the placeholder text changes appropriately as the workflow progresses.

### 3. Enhanced Welcome Message
**File:** `src/tui/app.rs`

**Before:**
```rust
"Welcome to Oxidized Bio Research Agent!\n\n\
 API Status: {} | {}\n\n\
 Paste a dataset path to begin, or /help for commands.\n\
 Press Ctrl+S to configure your API keys in Settings."
```

**After:**
```rust
"Welcome to Oxidized Bio Research Agent!\n\n\
 API Status: {} | {}\n\n\
 🔬 AUTOMATED WORKFLOW\n\
 Paste a dataset path (.csv or .tsv) to begin automated analysis:\n\
 → Upload → Plan → Literature → Findings → Drafts 1-3 → LaTeX\n\n\
 Requirements: Dataset must include Ensembl ID and Age columns.\n\n\
 Commands: Type /help for manual commands | Ctrl+S for Settings"
```

This makes the automated workflow more prominent and clearly explains:
- What file types are accepted (.csv or .tsv)
- The workflow stages
- The dataset requirements (Ensembl ID + Age columns)

## Automated Workflow Verification

The automated workflow is fully implemented and will trigger when:
1. `auto_mode` is enabled (default: `true`)
2. `workflow_stage` is `Upload`
3. User pastes a valid dataset path

The workflow will automatically execute:
- **Upload**: Load and validate the dataset
- **Plan**: Generate research plan for aging biomarker discovery
- **Literature**: Execute literature review tasks
- **Findings**: Run statistical analysis and generate findings
- **Draft 1-3**: Create three manuscript drafts
- **LaTeX**: Generate LaTeX output for publication

## Testing
Run the application:
```bash
cargo run
```

You should see:
1. A clear welcome message explaining the automated workflow
2. Input placeholder: "Paste dataset path (CSV/TSV with Ensembl ID + Age columns)..."
3. When you paste a valid dataset path, the automated workflow will begin

## Requirements
Dataset must be a CSV or TSV file with:
- Ensembl ID column (e.g., "ensembl_id", "Ensembl ID", etc.)
- Age column (e.g., "age", "Age", etc.)
