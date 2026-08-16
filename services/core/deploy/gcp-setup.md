# One-time GCP setup for the Cloud Run deploy pipeline

This has to be run once, by you, authenticated as yourself (`gcloud auth
login`) - it can't be done from the GitHub Actions pipeline itself, and it
shouldn't be automated by anything that doesn't already have your own GCP
credentials. Everything below is idempotent-ish (safe to re-run), and none
of it costs anything at this scale (Cloud Run, Artifact Registry, Secret
Manager, and Cloud Storage all have an always-free monthly allowance well
above what a handful of small-business tenants will use).

Project: **`bubbly-domain-288704`** (project number `482825254495` - shown
in the GCP console as "colmado-pos-prod", but that's just the display Name;
the ID and number below are the real, immutable identifiers everything
actually resolves against).
Region: **`us-east1`** (South Carolina - closer to the Dominican Republic
than the default `us-central1`, for lower latency to real users).

## 1. Project and APIs

```bash
gcloud config set project bubbly-domain-288704

gcloud services enable \
  run.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  storage.googleapis.com \
  iamcredentials.googleapis.com
```

If this fails complaining about billing, link a billing account to the
project first (console.cloud.google.com → Billing) - that step needs your
own payment details entered directly in Google's console, not something
that can be scripted here.

## 2. Artifact Registry (Docker image storage)

```bash
gcloud artifacts repositories create fiscal-core \
  --repository-format=docker \
  --location=us-east1 \
  --description="Colmado POS backend images"
```

## 3. Uploads bucket (fixes Cloud Run's ephemeral filesystem)

Cloud Run containers don't have persistent local disk - product photos and
logos would vanish on every restart/redeploy without this. The app's
`UPLOADS_DIR` env var points at this bucket, mounted as a regular directory,
so no code changes were needed for this fix.

```bash
gcloud storage buckets create gs://bubbly-domain-288704-fiscal-core-uploads \
  --location=us-east1 \
  --uniform-bucket-level-access
```

## 4. Two service accounts, not one

Kept separate on purpose: the deploy identity (what GitHub Actions acts as)
only needs to push images and trigger deploys - it should never be able to
read the app's own runtime secrets or touch the uploads bucket. The runtime
identity (what the running container acts as) is the reverse - it needs the
secrets and bucket access, but has no deploy permissions at all. Neither one
can do what the other does.

```bash
# Deploy identity - used by GitHub Actions
gcloud iam service-accounts create gh-deployer \
  --display-name="GitHub Actions deployer"

gcloud projects add-iam-policy-binding bubbly-domain-288704 \
  --member="serviceAccount:gh-deployer@bubbly-domain-288704.iam.gserviceaccount.com" \
  --role="roles/run.admin"

gcloud projects add-iam-policy-binding bubbly-domain-288704 \
  --member="serviceAccount:gh-deployer@bubbly-domain-288704.iam.gserviceaccount.com" \
  --role="roles/artifactregistry.writer"

gcloud projects add-iam-policy-binding bubbly-domain-288704 \
  --member="serviceAccount:gh-deployer@bubbly-domain-288704.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"

gcloud projects add-iam-policy-binding bubbly-domain-288704 \
  --member="serviceAccount:gh-deployer@bubbly-domain-288704.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

# Runtime identity - what the deployed container actually runs as
gcloud iam service-accounts create fiscal-core-runtime \
  --display-name="fiscal-core runtime"

gcloud projects add-iam-policy-binding bubbly-domain-288704 \
  --member="serviceAccount:fiscal-core-runtime@bubbly-domain-288704.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"

gcloud storage buckets add-iam-policy-binding gs://bubbly-domain-288704-fiscal-core-uploads \
  --member="serviceAccount:fiscal-core-runtime@bubbly-domain-288704.iam.gserviceaccount.com" \
  --role="roles/storage.objectAdmin"
```

## 5. Workload Identity Federation (this replaces a stored JSON key)

```bash
gcloud iam workload-identity-pools create github \
  --location="global" \
  --display-name="GitHub Actions"

gcloud iam workload-identity-pools providers create-oidc github-actions \
  --location="global" \
  --workload-identity-pool="github" \
  --display-name="GitHub Actions OIDC" \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --attribute-condition="assertion.repository == 'EmmanuelMat/retailmanagement-api'" \
  --issuer-uri="https://token.actions.githubusercontent.com"
```

The `--attribute-condition` above is what makes this safe - it's the line
that says "only THIS exact GitHub repo can ever use this trust relationship,"
so a workflow in any other repo (yours or anyone else's) can't impersonate
this service account even if they knew the pool name.

```bash
gcloud iam service-accounts add-iam-policy-binding \
  gh-deployer@bubbly-domain-288704.iam.gserviceaccount.com \
  --role="roles/iam.workloadIdentityUser" \
  --member="principalSet://iam.googleapis.com/projects/482825254495/locations/global/workloadIdentityPools/github/attribute.repository/EmmanuelMat/retailmanagement-api"
```

## 6. App secrets in Secret Manager

Generate the random ones yourself and pipe them straight in - they should
never exist as plaintext in a file, a chat message, or a shell history
entry beyond this one command.

```bash
openssl rand -base64 32 | gcloud secrets create JWT_SECRET --data-file=-
openssl rand -base64 32 | gcloud secrets create VENDOR_ADMIN_SECRET --data-file=-
openssl rand -base64 32 | gcloud secrets create LICENSE_SECRET --data-file=-
openssl rand -base64 32 | gcloud secrets create CERT_ENCRYPTION_KEY --data-file=-

# Your Neon connection string (from the Neon dashboard)
echo -n "postgres://USER:PASSWORD@HOST/DBNAME?sslmode=require" | \
  gcloud secrets create DATABASE_URL --data-file=-

# Optional - only needed if you want password-reset emails to actually send
# (self-service /olvide-password flow; the staff-side no-email reset still
# works without this). Leave this one out entirely if you're not using
# Resend yet - the app already handles a missing key gracefully.
echo -n "re_your_resend_key" | gcloud secrets create RESEND_API_KEY --data-file=-
```

## 7. GitHub repo configuration

Go to the repo's **Settings → Secrets and variables → Actions → Variables**
tab (not Secrets - none of this is sensitive, it's just identifiers; WIF
means no GCP credential ever needs to be stored in GitHub at all) and add:

| Variable | Value |
|---|---|
| `GCP_PROJECT_ID` | `bubbly-domain-288704` |
| `GCP_REGION` | `us-east1` |
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | `projects/482825254495/locations/global/workloadIdentityPools/github/providers/github-actions` |
| `GCP_SERVICE_ACCOUNT_EMAIL` | `gh-deployer@bubbly-domain-288704.iam.gserviceaccount.com` |
| `GCP_RUNTIME_SERVICE_ACCOUNT_EMAIL` | `fiscal-core-runtime@bubbly-domain-288704.iam.gserviceaccount.com` |
| `ARTIFACT_REGISTRY_REPO` | `fiscal-core` |
| `CLOUD_RUN_SERVICE` | `fiscal-core-backend` |
| `UPLOADS_BUCKET` | `bubbly-domain-288704-fiscal-core-uploads` |
| `DGII_ENV` | `TesteCF` (DGII sandbox - switch to the production value yourself once you're actually ready to file real e-CF documents, don't default to it) |
| `FRONTEND_URL` | your Vercel URL, e.g. `https://colmado-pos.vercel.app` |

Then go to **Settings → Environments**, create one named `production` (the
workflow deploys through it) - this also gives you a place to add required
reviewers later if you ever want a manual approval gate before deploy.

## What's intentionally not covered here

- The AI assistant module (`OLLAMA_URL`/`OLLAMA_MODEL`) needs an Ollama
  instance reachable from Cloud Run - that's a separate piece of
  infrastructure this pipeline doesn't stand up. The module just won't be
  available until that's solved; nothing breaks by leaving it unset.
- `CORE_GRPC_PORT` from `.env.example` isn't wired into this deploy -
  Cloud Run only exposes one public port per service, and there was no
  evidence in the codebase that anything currently depends on the gRPC
  port being externally reachable.

## After this is done

Merge to `main` (the workflow only triggers on `main`, plus manual runs via
the Actions tab) and the `deploy-cloud-run` workflow handles the rest -
build, migrate, deploy.
