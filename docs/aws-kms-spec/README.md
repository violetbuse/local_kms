# AWS KMS API — protocol notes for local_kms

Condensed from `docs.aws.amazon.com/kms/latest/APIReference/` (fetched 2026-09-16),
scoped to the actions this mock needs for envelope encryption:
[GenerateDataKey](generate-data-key.md), [Decrypt](decrypt.md), [CreateKey](create-key.md),
[CreateAlias](create-alias.md).

## Wire protocol

KMS uses AWS's "JSON RPC"-style protocol (`application/x-amz-json-1.1`), not REST-with-paths.
Every action is `POST /` to the same endpoint; the action is selected entirely by a header.

```
POST / HTTP/1.1
Host: kms.<region>.amazonaws.com
Content-Type: application/x-amz-json-1.1
X-Amz-Target: TrentService.<ActionName>
X-Amz-Date: <ISO8601 basic>
Authorization: AWS4-HMAC-SHA256 Credential=..., SignedHeaders=..., Signature=...

{ ...action-specific JSON body... }
```

- **`X-Amz-Target`**: `TrentService.<ActionName>` — e.g. `TrentService.GenerateDataKey`,
  `TrentService.Decrypt`, `TrentService.CreateKey`, `TrentService.CreateAlias`.
  `TrentService` is KMS's internal service name (a historical artifact); the mock should
  dispatch by splitting on `.` and matching the suffix.
- **Response**: HTTP 200 + JSON body for success. Errors are HTTP 4xx/5xx with a JSON body
  shaped like `{"__type": "com.amazonaws.kms#<ExceptionName>", "message": "..."}` and the
  `X-Amzn-ErrorType` header often set to the same exception name.
- **Binary fields** (`CiphertextBlob`, `Plaintext`, etc.) are base64-encoded strings over HTTP.
- **`Authorization` / SigV4`**: real KMS requires a valid SigV4 signature. For a local dev
  stub, we do **not** need to verify it — aws4fetch (or any SDK) will still sign requests,
  but nothing requires the server to check the signature. Just ignore the `Authorization`
  header and read `X-Amz-Target` + the JSON body.
- **Request ID**: real KMS returns `x-amzn-RequestId` on every response. Cheap to fake with
  a random UUID per request; not required for correctness but some SDKs log/expect it.

## Actions implemented by this mock

| Action | Purpose | Doc |
|---|---|---|
| `CreateKey` | Create a symmetric CMK (key material generated locally) | [create-key.md](create-key.md) |
| `CreateAlias` | Give a CMK a friendly `alias/name` | [create-alias.md](create-alias.md) |
| `GenerateDataKey` | Mint a random AES data key, return plaintext + ciphertext-under-CMK | [generate-data-key.md](generate-data-key.md) |
| `Decrypt` | Decrypt a `CiphertextBlob` produced by `GenerateDataKey` (or `Encrypt`) | [decrypt.md](decrypt.md) |

Out of scope for the mock (not needed for envelope encryption): asymmetric keys, HMAC keys,
multi-Region keys, custom key stores, grants, key policies/IAM, tagging, Nitro
Enclave `Recipient` attestation, `DryRun`.

## Minimal semantics to replicate

- `KeyId` accepts: raw key ID, key ARN, `alias/Name`, or alias ARN. The mock should resolve
  all four to the same underlying key-material record.
- `GenerateDataKey` requires exactly one of `KeySpec` (`AES_256` | `AES_128`) or
  `NumberOfBytes` (1–1024) — reject if both or neither are set.
- `EncryptionContext` (string→string map) is AAD: the same map must be supplied on `Decrypt`
  as was supplied on `GenerateDataKey`/`Encrypt`, or decryption must fail
  (`InvalidCiphertextException`) — this is exactly Web Crypto AES-GCM's `additionalData`.
- `Decrypt`'s `KeyId` is optional when the ciphertext was made with a symmetric key, because
  real KMS embeds the key identity in the ciphertext blob's metadata. Easiest local
  implementation: embed the key ID (and IV/nonce, and material id) in the blob you construct,
  so `Decrypt` can self-describe.
- Successful `CreateAlias` returns HTTP 200 with an **empty body** (no JSON at all).
