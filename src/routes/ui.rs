use axum::{response::Html, Router, routing::get};

pub fn router() -> Router {
    Router::new().route("/", get(index))
}

async fn index() -> Html<&'static str> {
    Html(r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Oxidized Bio - Biomarker Analysis</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 2rem; color: #1d1d1f; }
    h1 { margin-bottom: 0.5rem; }
    .card { border: 1px solid #ddd; padding: 1rem; border-radius: 8px; margin-bottom: 1rem; }
    label { display: block; margin-top: 0.75rem; font-weight: 600; }
    input, textarea { width: 100%; padding: 0.5rem; }
    button { margin-top: 1rem; padding: 0.6rem 1rem; }
    pre { background: #f6f8fa; padding: 1rem; overflow: auto; }
  </style>
</head>
<body>
  <h1>Oxidized Bio</h1>
  <p>Upload a CSV/TSV, then run biomarker analysis and receive a manuscript-style summary.</p>

  <div class="card">
    <h2>1) Upload dataset</h2>
    <input id="fileInput" type="file" />
    <button id="uploadBtn">Upload</button>
    <div id="uploadStatus"></div>
  </div>

  <div class="card">
    <h2>2) Run analysis</h2>
    <label>Dataset ID</label>
    <input id="datasetId" placeholder="Paste dataset id from upload response" />
    <label>Target column (e.g., age)</label>
    <input id="targetColumn" value="age" />
    <label>Group column (e.g., cell_type)</label>
    <input id="groupColumn" value="cell_type" />
    <label>Covariates (comma separated)</label>
    <input id="covariates" placeholder="batch, sex" />
    <label>Boxplot column</label>
    <input id="boxplotColumn" placeholder="marker_1" />
    <button id="analyzeBtn">Run analysis</button>
  </div>

  <div class="card">
    <h2>Output</h2>
    <pre id="output"></pre>
  </div>

  <script>
    const uploadBtn = document.getElementById('uploadBtn');
    const analyzeBtn = document.getElementById('analyzeBtn');
    const output = document.getElementById('output');
    const uploadStatus = document.getElementById('uploadStatus');

    uploadBtn.addEventListener('click', async () => {
      const fileInput = document.getElementById('fileInput');
      if (!fileInput.files.length) {
        uploadStatus.textContent = 'Select a file first.';
        return;
      }
      const formData = new FormData();
      formData.append('file', fileInput.files[0]);
      uploadStatus.textContent = 'Uploading...';
      const res = await fetch('/api/files', { method: 'POST', body: formData });
      const json = await res.json();
      uploadStatus.textContent = JSON.stringify(json, null, 2);
      if (json?.dataset?.id) {
        document.getElementById('datasetId').value = json.dataset.id;
      }
    });

    analyzeBtn.addEventListener('click', async () => {
      const datasetId = document.getElementById('datasetId').value.trim();
      if (!datasetId) {
        output.textContent = 'Provide a dataset id.';
        return;
      }
      const covariates = document.getElementById('covariates').value
        .split(',')
        .map(s => s.trim())
        .filter(Boolean);
      const payload = {
        dataset_id: datasetId,
        target_column: document.getElementById('targetColumn').value.trim() || null,
        group_column: document.getElementById('groupColumn').value.trim() || null,
        covariates: covariates.length ? covariates : null,
        boxplot_column: document.getElementById('boxplotColumn').value.trim() || null
      };
      output.textContent = 'Running analysis...';
      const res = await fetch('/api/analysis', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
      const json = await res.json();
      output.textContent = JSON.stringify(json, null, 2);
    });
  </script>
</body>
</html>"#)
}
