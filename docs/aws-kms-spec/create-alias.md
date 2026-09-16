# CreateAlias

`X-Amz-Target: TrentService.CreateAlias`

Attaches a friendly `alias/Name` to a CMK, so `KeyId` params elsewhere can use the alias
instead of the raw key ID/ARN.

Source: <https://docs.aws.amazon.com/kms/latest/APIReference/API_CreateAlias.html>

## Request

```jsonc
{
   "AliasName": "string",     // required — must start with "alias/", pattern ^alias/[a-zA-Z0-9/_-]+$
   "TargetKeyId": "string"    // required — key ID or key ARN (not another alias) of an existing key
}
```

- `alias/aws/...` is reserved for AWS-managed keys in real KMS — reasonable to reject in the
  mock too, but not load-bearing for envelope encryption.
- Alias names must be unique per account/region — the mock's store should enforce uniqueness
  within its single namespace.

## Response

**Empty body, HTTP 200.** No JSON at all — this is the one action here with no response
payload. (To retrieve what you created, real KMS expects a separate `ListAliases` call; the
mock's internal alias→key-id map can just be queried directly wherever `KeyId` resolution
happens.)

## Errors

`AlreadyExistsException` (400 — alias name taken), `InvalidAliasNameException` (400 — bad
format), `NotFoundException` (400 — `TargetKeyId` doesn't exist), `KMSInvalidStateException`
(400), `LimitExceededException` (400), `DependencyTimeoutException` (500),
`KMSInternalException` (500).

## Example

```
POST / HTTP/1.1
X-Amz-Target: TrentService.CreateAlias
Content-Type: application/x-amz-json-1.1

{
  "TargetKeyId": "1234abcd-12ab-34cd-56ef-1234567890ab",
  "AliasName": "alias/ExampleAlias"
}
```

```
HTTP/1.1 200 OK
Content-Length: 0
```
