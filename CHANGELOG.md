# Changelog

Every release of kubelint, newest first. The nightly release workflow writes this file.

Older releases are listed on the [releases page](https://github.com/YassWorks/kubelint/releases).

## [0.22.2] - 2026-11-22

### Fixes

- check KL1008 Service and ReplicationController names for the selected Kubernetes version (#150)

### Other changes

- check KL1001 and KL1002 against kubectl and the API server (#149)
- check KL1003, KL1008 and KL1010 against the API server (#148)

## [0.22.1] - 2026-11-20

### Other changes

- check KL1005, KL1006, KL1007, KL1009 and KL1011 against the API server (#146)
- check the Pod Security rule fixtures against a real API server (#144)

## [0.22.0] - 2026-11-19

### Features

- add --kubernetes-version and the kubernetes-version key (#142)
- embed the schemas of Kubernetes 1.34 to 1.37 (#141)
- import a Kubernetes version's OpenAPI schemas (#140)

## [0.21.1] - 2026-11-18

### Other changes

- publish a container image to ghcr.io (#138)

## [0.21.0] - 2026-11-18

### Features

- add kubelint self update (#135)

### Other changes

- publish a Homebrew formula (#137)

## [0.20.3] - 2026-11-17

### Other changes

- build and publish releases with dist (#133)
