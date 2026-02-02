use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::UploadedDataset;

#[derive(Debug, Clone)]
pub struct DatasetRecord {
    pub dataset: UploadedDataset,
    pub local_path: String,
    pub content_type: String,
    pub delimiter: u8,
    pub has_headers: bool,
    pub columns: Vec<String>,
    pub row_count: usize,
}

#[derive(Clone, Default)]
pub struct DatasetRegistry {
    inner: Arc<RwLock<HashMap<String, DatasetRecord>>>,
}

impl DatasetRegistry {
    pub async fn insert(&self, record: DatasetRecord) {
        let mut guard = self.inner.write().await;
        guard.insert(record.dataset.id.clone(), record);
    }

    pub async fn get(&self, dataset_id: &str) -> Option<DatasetRecord> {
        let guard = self.inner.read().await;
        guard.get(dataset_id).cloned()
    }
}
