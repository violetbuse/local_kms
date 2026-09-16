# GenerateDataKey

`X-Amz-Target: TrentService.GenerateDataKey`

Returns a unique symmetric data key: a plaintext copy (for the caller to use immediately and
then discard) and an encrypted copy (`CiphertextBlob`, encrypted under the specified CMK, safe
to store alongside the ciphertext it protects).

Source: <https://docs.aws.amazon.com/kms/latest/APIReference/API_GenerateDataKey.html>

## Request

```jsonc
{
   "KeyId": "string",                 // required — key ID, key ARN, "alias/Name", or alias ARN
   "KeySpec": "AES_256" | "AES_128",  // exactly one of KeySpec / NumberOfBytes
   "NumberOfBytes": 1-1024,
   "EncryptionContext": { "string": "string" },  // optional AAD map
   "GrantTokens": ["string"],         // ignore in mock
   "DryRun": boolean,                 // ignore in mock
   "Recipient": { ... }               // Nitro Enclave attestation — out of scope
}
```

- `KeyId` required. Must resolve to a symmetric encryption key in the mock's store.
- Exactly one of `KeySpec` or `NumberOfBytes` must be present (real KMS errors if both or
  neither are given — `ValidationException` in practice, not itemized on the errors page).

## Response

```jsonc
{
   "CiphertextBlob": "base64",   // data key encrypted under the CMK
   "Plaintext": "base64",        // raw data key bytes — caller uses this, then discards it
   "KeyId": "string",            // ARN of the CMK used
   "KeyMaterialId": "string"     // identifies which key-material version was used (64 hex chars)
}
```

`CiphertextForRecipient` (Nitro Enclave only) is out of scope for the mock.

## Mock implementation notes

1. Generate `NumberOfBytes` (or 16/32 per `KeySpec`) random bytes → this is `Plaintext`.
2. AES-GCM-encrypt those bytes under the CMK's local raw key material, using
   `EncryptionContext` (if present) as additional authenticated data.
3. `CiphertextBlob` = base64(IV/nonce ‖ ciphertext ‖ auth tag ‖ enough metadata for `Decrypt`
   to find the right key without being told `KeyId` again — e.g. prefix with the key's ID).
   Real KMS's blob format is opaque/undocumented; the mock only needs its own format to be
   self-consistent between `GenerateDataKey` and `Decrypt`.

## Errors (shape only — mock can return a generic 400 for unknowns)

`DependencyTimeoutException` (500), `DisabledException` (400), `DryRunOperationException`
(400), `InvalidGrantTokenException` (400), `InvalidKeyUsageException` (400),
`KeyUnavailableException` (500), `KMSInternalException` (500), `KMSInvalidStateException`
(400), `NotFoundException` (400, unknown `KeyId`).

## Example

```
POST / HTTP/1.1
X-Amz-Target: TrentService.GenerateDataKey
Content-Type: application/x-amz-json-1.1

{ "KeyId": "alias/ExampleAlias", "KeySpec": "AES_256" }
```

```json
{
  "CiphertextBlob": "AQEDAHjRYf5W...",
  "KeyId": "arn:aws:kms:us-east-2:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab",
  "KeyMaterialId": "0b7fd7ddbac6eef27907413567cad8c810e2883dc8a7534067a82ee1142fc1e6",
  "Plaintext": "VdzKNHGzUAzJeRBVY+uUmofUGGiDzyB3+i9fVkh3piw="
}
```
