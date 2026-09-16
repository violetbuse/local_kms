# syntax=docker/dockerfile:1
#
# Built from pre-compiled release binaries (see .github/workflows/release.yml), not from
# source, so this expects a `dist/linux-<amd64|arm64>/local_kms` binary in the build context
# matching buildx's TARGETARCH for each platform in the multi-arch build.
FROM debian:bookworm-slim

ARG TARGETARCH
COPY --chmod=755 dist/linux-${TARGETARCH}/local_kms /usr/local/bin/local_kms

ENV LOCAL_KMS_DB_PATH=/data/persistence.db
VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/local_kms"]
