# Xavier2 on Google Cloud Free Tier

Deploy Xavier2 to Google Cloud using free-tier eligible services.

## Free Tier Resources

| Service | Free Tier | Notes |
|---------|-----------|-------|
| **Cloud Run** | 2 million requests/month | CPU + memory always free |
| **Cloud Storage** | 5GB | Standard storage |
| **Cloud SQL** | 1 instance | 30 days free trial |
| **Artifact Registry** | 0.5GB | Docker images |

---

## Option 1: Cloud Run (Recommended)

### 1. Build and Push

```bash
# Authenticate
gcloud auth configure-docker

# Build image
docker build -t xavier2 .

# Tag for Artifact Registry
docker tag xavier2 gcr.io/[PROJECT-ID]/xavier2:v0.4.1

# Push
docker push gcr.io/[PROJECT-ID]/xavier2:v0.4.1
```

### 2. Deploy to Cloud Run

```bash
gcloud run deploy xavier2 \
  --image gcr.io/[PROJECT-ID]/xavier2:v0.4.1 \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --port 8003 \
  --memory 512Mi \
  --cpu 1
```

### 3. Set Environment Variables

```bash
gcloud run services update xavier2 \
  --set-env-vars "XAVIER2_TOKEN=your-secure-token" \
  --region us-central1
```

---

## Option 2: App Engine Flex

### app.yaml
```yaml
runtime: custom
env: flex

resources:
  cpu: 1
  memory_gb: 0.5

manual_scaling:
  instances: 1

automatic_scaling:
  min_instances: 0
  max_instances: 1
```

Deploy:
```bash
gcloud app deploy
```

---

## Persistence Options

### Local (Free)
- Use SQLite or file storage in container
- No additional cost
- Data persists until container restart

### Cloud Storage (~$0.02/GB/month)
```bash
# Mount GCS bucket as volume
gcloud run deploy xavier2 \
  --mount-memory=512Mi \
  --volume "xavier2-data=/data" \
  ...
```

### Cloud SQL (First 90 days free)
```bash
# Create instance
gcloud sql instances create xavier2-db \
  --database-version=MYSQL_8_0 \
  --tier=db-f1-micro \
  --region=us-central1

# Connect from Cloud Run
gcloud run deploy xavier2 \
  --add-cloudsql-instances=[PROJECT-ID]:us-central1:xavier2-db
```

---

## Custom Domain (Free)

```bash
gcloud run domain-mappings create --service xavier2 --domain xavier2.yourdomain.com
```

SSL is automatic with Cloud Run.

---

## Cost Estimation

### Free Tier Usage
| Resource | Usage | Cost |
|----------|-------|------|
| Cloud Run | 2M reqs, 180K CPU-seconds | **$0.00** |
| Cloud Storage | 1GB | **$0.00** |
| Egress | ~10GB/month | **~$0.12** |
| **Total** | | **~$0.12/month** |

### With Paid Tier ($8/month)
| Add-on | Cost |
|--------|------|
| Extra storage (10GB) | Included |
| Custom domain SSL | Included |
| Priority support | Included |
| **Total** | **$8.00/month** |

---

## Dockerfile for Cloud

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features ci-safe

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/xavier2 /usr/local/bin/
EXPOSE 8003
CMD ["xavier2"]
```

---

## Health Check

```bash
curl https://[YOUR-REGION]-[PROJECT-ID].run.app/health
```

Response:
```json
{"status":"ok","service":"xavier2","version":"0.4.1"}
```

---

## Monitoring (Free)

```bash
# View logs
gcloud run logs xavier2 --region us-central1

# View metrics
gcloud monitoring dashboard create --config-from-file=dashboard.json
```

---

## Troubleshooting

### Container exceeds memory
```bash
gcloud run services update xavier2 \
  --memory 1Gi \
  --region us-central1
```

### Cold starts slow
- Use min instances: 1 (costs ~$6/month)
- Or accept 1-2s cold start

### Region not supported
Use `us-central1` — most stable free tier region.

---

## Next Steps

1. Set up custom domain
2. Configure Cloud SQL for production data
3. Enable Cloud Armor for DDoS protection
4. Set up monitoring dashboards
