# Decrypt

`X-Amz-Target: TrentService.Decrypt`

Decrypts a `CiphertextBlob` produced by `Encrypt` or `GenerateDataKey` (any of the
`GenerateDataKey*` variants). This is the inverse of [generate-data-key.md](generate-data-key.md).

Source: <https://docs.aws.amazon.com/kms/latest/APIReference/API_Decrypt.html>

## Request

```jsonc
{
   "CiphertextBlob": "base64",       // required (unless DryRun+IGNORE_CIPHERTEXT, n/a to mock)
   "KeyId": "string",                // optional for symmetric ciphertexts — see below
   "EncryptionContext": { "string": "string" },  // must exactly match what encryption used
   "EncryptionAlgorithm": "SYMMETRIC_DEFAULT",   // default; only matters for asymmetric keys
   "GrantTokens": ["string"],        // ignore in mock
   "DryRun": boolean                 // ignore in mock
}
```

- `KeyId` is optional for symmetric ciphertext blobs because real KMS embeds key identity in
  the blob's metadata. The mock should do the same (see generate-data-key.md's implementation
  note) so `Decrypt` can work without `KeyId`. If `KeyId` **is** given and doesn't match the
  key actually used to encrypt, real KMS throws `IncorrectKeyException` — worth replicating
  since it's a real safety check apps may rely on.
- `EncryptionContext` must match byte-for-byte (case-sensitive) what was passed to
  `GenerateDataKey`/`Encrypt`, else decryption must fail — this falls straight out of using it
  as AES-GCM additional authenticated data; a mismatch fails AEAD tag verification.

## Response

```jsonc
{
   "Plaintext": "base64",            // decrypted bytes
   "KeyId": "string",                // ARN of the CMK actually used
   "KeyMaterialId": "string",
   "EncryptionAlgorithm": "SYMMETRIC_DEFAULT"
}
```

## Errors

`DependencyTimeoutException` (500), `DisabledException` (400), `DryRunOperationException`
(400), `IncorrectKeyException` (400 — `KeyId` given doesn't match blob's actual key),
`InvalidCiphertextException` (400 — corrupt/tampered blob, or wrong `EncryptionContext`; maps
to an AES-GCM auth-tag verification failure), `InvalidGrantTokenException` (400),
`InvalidKeyUsageException` (400), `KeyUnavailableException` (500), `KMSInternalException`
(500), `KMSInvalidStateException` (400), `NotFoundException` (400).

## Example

```
POST / HTTP/1.1
X-Amz-Target: TrentService.Decrypt
Content-Type: application/x-amz-json-1.1

{
  "KeyId": "arn:aws:kms:us-west-2:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab",
  "CiphertextBlob": "CiDPoCH188S65r5Cy7pAhIFJMXDlU7mewhSlYUpuQIVBg=="
}
```

```json
{
  "KeyId": "arn:aws:kms:us-west-2:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab",
  "KeyMaterialId": "0b7fd7ddbac6eef27907413567cad8c810e2883dc8a7534067a82ee1142fc1e6",
  "Plaintext": "VGhpcyBpcyBEYXkgMSBmb3IgdGhlIEludGVybmV0Cg==",
  "EncryptionAlgorithm": "SYMMETRIC_DEFAULT"
}
```
