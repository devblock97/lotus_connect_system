use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use config_crate::AppConfig;
use errors::{AppError, Result};

#[async_trait::async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload_file(&self, file_name: &str, data: Vec<u8>) -> Result<String>;
    async fn delete_file(&self, file_path: &str) -> Result<()>;
}

pub struct LocalStorageProvider {
    upload_dir: PathBuf,
}

impl LocalStorageProvider {
    pub fn new(upload_dir: &str) -> Self {
        Self {
            upload_dir: PathBuf::from(upload_dir),
        }
    }
}

#[async_trait::async_trait]
impl StorageProvider for LocalStorageProvider {
    async fn upload_file(&self, file_name: &str, data: Vec<u8>) -> Result<String> {
        // Ensure upload directory exists
        if !self.upload_dir.exists() {
            fs::create_dir_all(&self.upload_dir)
                .await
                .map_err(|err| AppError::Internal(format!("Failed to create upload directory: {}", err)))?;
        }

        // Generate a unique time-ordered file name to prevent collision
        let unique_name = format!("{}-{}", Uuid::now_v7(), file_name);
        let dest_path = self.upload_dir.join(&unique_name);

        fs::write(&dest_path, data)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to write file locally: {}", err)))?;

        Ok(dest_path.to_string_lossy().to_string())
    }

    async fn delete_file(&self, file_path: &str) -> Result<()> {
        let path = PathBuf::from(file_path);
        if path.exists() {
            fs::remove_file(path)
                .await
                .map_err(|err| AppError::Internal(format!("Failed to delete file: {}", err)))?;
        }
        Ok(())
    }
}

pub struct S3StorageProvider {
    bucket: String,
    endpoint: String,
}

impl S3StorageProvider {
    pub fn new(bucket: String, endpoint: String) -> Self {
        Self { bucket, endpoint }
    }
}

#[async_trait::async_trait]
impl StorageProvider for S3StorageProvider {
    async fn upload_file(&self, file_name: &str, data: Vec<u8>) -> Result<String> {
        // Simulate S3 object upload
        tracing::info!("Uploading {} bytes of file {} to S3 bucket {}", data.len(), file_name, self.bucket);
        
        let unique_name = format!("{}-{}", Uuid::now_v7(), file_name);
        let s3_url = format!("{}/{}/{}", self.endpoint, self.bucket, unique_name);
        Ok(s3_url)
    }

    async fn delete_file(&self, file_path: &str) -> Result<()> {
        tracing::info!("Deleting file {} from S3 bucket", file_path);
        Ok(())
    }
}

/// Factory function to return dynamic StorageProvider implementation based on AppConfig
pub fn create_storage_provider(config: &AppConfig) -> Arc<dyn StorageProvider> {
    if let (Some(bucket), Some(endpoint)) = (&config.s3_bucket, &config.s3_endpoint) {
        Arc::new(S3StorageProvider::new(bucket.clone(), endpoint.clone()))
    } else {
        Arc::new(LocalStorageProvider::new(&config.upload_dir))
    }
}
