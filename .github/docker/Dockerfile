# The build context holds amd64/kubelint and arm64/kubelint (.github/scripts/publish-ghcr.sh).
FROM alpine:3.24
ARG TARGETARCH
COPY ${TARGETARCH}/kubelint /usr/local/bin/kubelint
WORKDIR /src
ENTRYPOINT ["kubelint"]
