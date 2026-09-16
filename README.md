# local_kms

A local mock of the AWS KMS HTTP API, for developing envelope encryption without needing real
AWS credentials or infrastructure. Point any AWS SDK at it and it just works — SigV4 signatures
are accepted but never verified, so any (even fake) credentials are fine.

It implements the four KMS actions envelope encryption actually needs: `CreateKey`,
`CreateAlias`, `GenerateDataKey`, and `Decrypt`. Keys are real AES-256-GCM key material,
persisted to a local SQLite file, so data keys generated in one run can be decrypted in the
next. See [`docs/aws-kms-spec`](docs/aws-kms-spec) for the protocol details this replicates.

## Quick start

**Install a binary:**

```sh
curl -fsSL https://raw.githubusercontent.com/violetbuse/local_kms/master/install.sh | bash
local_kms
```

Installs to `/usr/local/bin` (re-run the same command any time to update). See
[`install.sh`](install.sh) for override env vars.

**Or run the Docker image:**

```sh
docker run -p 8080:8080 -v kms-data:/data ghcr.io/violetbuse/local_kms:latest
```

Either way, the server listens on `:8080` and speaks the same wire protocol as
`kms.<region>.amazonaws.com`.

## Pointing an SDK at it

Override the endpoint and use any credentials — they're never checked:

```sh
export AWS_ENDPOINT_URL_KMS=http://localhost:8080
export AWS_ACCESS_KEY_ID=local
export AWS_SECRET_ACCESS_KEY=local
export AWS_REGION=us-east-2
```

```sh
aws --endpoint-url http://localhost:8080 kms create-key
```

## Example

```sh
# Create a key
curl -s http://localhost:8080/ \
  -H 'X-Amz-Target: TrentService.CreateKey' \
  -H 'Content-Type: application/x-amz-json-1.1' \
  -d '{}'

# Generate a data key
curl -s http://localhost:8080/ \
  -H 'X-Amz-Target: TrentService.GenerateDataKey' \
  -H 'Content-Type: application/x-amz-json-1.1' \
  -d '{"KeyId": "<KeyId from above>", "KeySpec": "AES_256"}'

# Decrypt it back
curl -s http://localhost:8080/ \
  -H 'X-Amz-Target: TrentService.Decrypt' \
  -H 'Content-Type: application/x-amz-json-1.1' \
  -d '{"CiphertextBlob": "<CiphertextBlob from above>"}'
```

## Configuration

| Env var | Default | Description |
|---|---|---|
| `LOCAL_KMS_DB_PATH` | `./.violet/kms-mock/persistence.db` | SQLite file path (the Docker image sets this to `/data/persistence.db`) |
| `LOCAL_KMS_PORT` | `8080` | Port to listen on |

## Development

```sh
cargo test    # unit + integration tests
cargo run     # start the server locally
```

Release binaries and the Docker image are built by
[`.github/workflows/release.yml`](.github/workflows/release.yml) on every `vX.Y.Z` tag push.
