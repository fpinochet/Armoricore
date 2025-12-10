# Akamai Object Storage Integration

The Media Processor uses **Akamai Object Storage** via S3-compatible API using the `rusoto_s3` library.

## Configuration

Akamai Object Storage is S3-compatible, so it works seamlessly with the AWS S3 SDK. Configure it using environment variables:

```bash
# Akamai Object Storage Configuration
OBJECT_STORAGE_ENDPOINT=https://your-bucket.akamai.com
OBJECT_STORAGE_ACCESS_KEY=your-akamai-access-key
OBJECT_STORAGE_SECRET_KEY=your-akamai-secret-key
OBJECT_STORAGE_BUCKET=your-bucket-name
OBJECT_STORAGE_REGION=akamai  # Optional, defaults to "akamai"
```

## Endpoint Formats

The implementation supports multiple endpoint formats:

1. **Full HTTPS URL** (Recommended):
   ```
   OBJECT_STORAGE_ENDPOINT=https://your-bucket.akamai.com
   ```

2. **S3-style URL** (Auto-converted):
   ```
   OBJECT_STORAGE_ENDPOINT=s3://your-bucket
   ```
   Will be converted to: `https://your-bucket.akamai.com`

3. **Bucket name only** (Auto-converted):
   ```
   OBJECT_STORAGE_ENDPOINT=your-bucket
   ```
   Will be converted to: `https://your-bucket.akamai.com`

## How It Works

The implementation uses `rusoto_s3` which is compatible with:
- ✅ AWS S3
- ✅ Akamai Object Storage
- ✅ MinIO
- ✅ Any S3-compatible storage

### Custom Region Configuration

For Akamai, we use a custom region with the Akamai endpoint:

```rust
Region::Custom {
    name: "akamai".to_string(),
    endpoint: "https://your-bucket.akamai.com",
}
```

This allows the AWS SDK to connect to Akamai's S3-compatible API.

## File Upload

Files are uploaded with:
- **Content-Type**: Set based on file type (video/mp4, image/jpeg, etc.)
- **Cache-Control**: `public, max-age=31536000` (1 year)
- **Public URLs**: Generated as `{endpoint}/{s3_key}`

### Example Upload

```rust
// Upload a file
let url = storage.upload_file(
    &local_path,
    "media/123/playlist.m3u8",
    "application/vnd.apple.mpegurl"
).await?;

// Result: https://your-bucket.akamai.com/media/123/playlist.m3u8
```

## CDN Integration

Akamai Object Storage is typically fronted by Akamai CDN. The URLs generated point directly to the object storage, which is then served through Akamai's CDN for optimal performance.

## Testing

To test the integration:

1. Set environment variables with your Akamai credentials
2. Run the media processor:
   ```bash
   cd rust-services
   cargo run --bin media-processor
   ```
3. Publish a `media.uploaded` event to NATS
4. Check logs for upload confirmation

## Troubleshooting

### Connection Issues
- Verify Akamai endpoint URL is correct
- Check credentials are valid
- Ensure bucket name matches

### Upload Failures
- Check file permissions
- Verify bucket exists and is accessible
- Check network connectivity to Akamai

### URL Generation
- URLs are generated as: `{endpoint}/{s3_key}`
- Ensure CDN is configured to serve from object storage
- Verify CORS settings if accessing from browser

## Production Considerations

1. **Credentials**: Use secure secret management (AWS Secrets Manager, HashiCorp Vault)
2. **Error Handling**: Implement retry logic for transient failures
3. **Monitoring**: Track upload success/failure rates
4. **Cost Optimization**: Use appropriate storage classes
5. **CDN Configuration**: Ensure Akamai CDN is properly configured

