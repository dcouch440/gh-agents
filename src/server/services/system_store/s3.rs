use anyhow::{Context, Result};
use aws_sdk_s3::primitives::ByteStream;

/// S3-compatible object storage backend.
///
/// Wraps the AWS SDK client. In dev this points at MinIO; in prod at real S3.
pub struct S3Backend {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3Backend {
    /// Create a new S3 backend.
    ///
    /// - `endpoint`: custom endpoint URL for MinIO (e.g. `http://localhost:9000`).
    ///   Pass `None` to use real AWS S3.
    /// - `bucket`: the bucket name. Created automatically if it doesn't exist.
    pub async fn new(endpoint: Option<&str>, bucket: &str) -> Result<Self> {
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        if let Some(ep) = endpoint {
            config_loader = config_loader.endpoint_url(ep);
        }

        let sdk_config = config_loader.load().await;

        let mut s3_config = aws_sdk_s3::config::Builder::from(&sdk_config);

        // MinIO requires path-style access and skipping the region check
        if endpoint.is_some() {
            s3_config = s3_config
                .force_path_style(true)
                .region(aws_sdk_s3::config::Region::new("us-east-1"));
        }

        let client = aws_sdk_s3::Client::from_conf(s3_config.build());

        // Ensure bucket exists
        let bucket_exists = client.head_bucket().bucket(bucket).send().await.is_ok();

        if !bucket_exists {
            client
                .create_bucket()
                .bucket(bucket)
                .send()
                .await
                .with_context(|| format!("failed to create S3 bucket '{bucket}'"))?;
            tracing::info!("Created S3 bucket: {bucket}");
        }

        Ok(Self {
            client,
            bucket: bucket.to_string(),
        })
    }

    /// Read an object's bytes by key.
    pub async fn read(&self, key: &str) -> Result<Vec<u8>> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("S3 get_object failed: {key}"))?;

        let bytes = output
            .body
            .collect()
            .await
            .with_context(|| format!("S3 read body failed: {key}"))?
            .into_bytes()
            .to_vec();

        Ok(bytes)
    }

    /// Write bytes to an object key.
    pub async fn write(&self, key: &str, bytes: &[u8], content_type: &str) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes.to_vec()))
            .content_type(content_type)
            .send()
            .await
            .with_context(|| format!("S3 put_object failed: {key}"))?;

        Ok(())
    }

    /// Delete a single object by key.
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("S3 delete_object failed: {key}"))?;

        Ok(())
    }

    /// Delete all objects whose key starts with the given prefix. Returns count deleted.
    pub async fn delete_prefix(&self, prefix: &str) -> Result<u64> {
        let mut count = 0u64;
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);

            if let Some(token) = &continuation_token {
                req = req.continuation_token(token);
            }

            let output = req
                .send()
                .await
                .with_context(|| format!("S3 list_objects_v2 failed: {prefix}"))?;

            let contents = output.contents();
            if contents.is_empty() {
                break;
            }

            for obj in contents {
                if let Some(key) = obj.key() {
                    self.delete(key).await?;
                    count += 1;
                }
            }

            if output.is_truncated() == Some(true) {
                continuation_token = output.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(count)
    }

    /// Check if an object exists.
    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => {
                let service_err = err.into_service_error();
                if service_err.is_not_found() {
                    Ok(false)
                } else {
                    Err(anyhow::anyhow!(
                        "S3 head_object failed: {key}: {service_err}"
                    ))
                }
            }
        }
    }
}
