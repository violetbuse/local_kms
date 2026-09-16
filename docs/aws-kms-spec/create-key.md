# CreateKey

`X-Amz-Target: TrentService.CreateKey`

Creates a new CMK (customer managed key). For the mock, only the **symmetric encryption key**
path matters — that's the default when no `KeySpec`/`KeyUsage` are given.

Source: <https://docs.aws.amazon.com/kms/latest/APIReference/API_CreateKey.html>

## Request (fields the mock cares about; everything else — asymmetric specs, custom key
stores, multi-Region, XKS, tags, policy — is safely ignorable)

```jsonc
{
   "Description": "string",           // optional, cosmetic
   "KeySpec": "SYMMETRIC_DEFAULT",     // default; mock only needs to support this value
   "KeyUsage": "ENCRYPT_DECRYPT",      // default; mock only needs to support this value
   "Origin": "AWS_KMS"                 // default: KMS (mock) generates the key material
}
```

If `KeySpec`/`KeyUsage` are anything other than the symmetric defaults, it's reasonable for
the mock to reject with `UnsupportedOperationException` rather than implement asymmetric/HMAC
key types.

## Response

Real KMS returns a large `KeyMetadata` object. The mock only needs to populate the fields an
app would plausibly read back:

```jsonc
{
  "KeyMetadata": {
    "KeyId": "string",                    // UUID-shaped key ID, generate locally
    "Arn": "string",                      // fake but well-formed: arn:aws:kms:<region>:<acct>:key/<KeyId>
    "AWSAccountId": "string",             // any fixed fake account id is fine
    "CreationDate": <epoch seconds>,
    "Enabled": true,
    "KeyState": "Enabled",
    "KeyUsage": "ENCRYPT_DECRYPT",
    "KeySpec": "SYMMETRIC_DEFAULT",
    "KeyManager": "CUSTOMER",
    "Origin": "AWS_KMS",
    "MultiRegion": false,
    "Description": "",
    "EncryptionAlgorithms": ["SYMMETRIC_DEFAULT"],
    "CurrentKeyMaterialId": "string"      // 64 hex chars, matches GenerateDataKey's KeyMaterialId
  }
}
```

## Errors

Real KMS has a long list (`MalformedPolicyDocumentException`, `TagException`,
`CustomKeyStore*`, `XksKey*`, etc.) that mostly concern features the mock doesn't implement.
Realistically only `KMSInternalException` (500) and `UnsupportedOperationException` (400, for
unsupported `KeySpec`/`Origin` values) are worth modeling.

## Example

```
POST / HTTP/1.1
X-Amz-Target: TrentService.CreateKey
Content-Type: application/x-amz-json-1.1

{}
```

```json
{
  "KeyMetadata": {
    "AWSAccountId": "111122223333",
    "Arn": "arn:aws:kms:us-east-2:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab",
    "CreationDate": "2025-04-16T16:03:04.060000-07:00",
    "Description": "",
    "Enabled": true,
    "KeyId": "1234abcd-12ab-34cd-56ef-1234567890ab",
    "KeyManager": "CUSTOMER",
    "KeySpec": "SYMMETRIC_DEFAULT",
    "KeyState": "Enabled",
    "KeyUsage": "ENCRYPT_DECRYPT",
    "MultiRegion": false,
    "Origin": "AWS_KMS",
    "EncryptionAlgorithms": ["SYMMETRIC_DEFAULT"],
    "CurrentKeyMaterialId": "0b7fd7ddbac6eef27907413567cad8c810e2883dc8a7534067a82ee1142fc1e6"
  }
}
```
