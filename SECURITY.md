# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
| < 0.1   | No        |

## Reporting a vulnerability

Please email **security@gravixlayer.ai** with:

- A description of the issue and impact
- Steps to reproduce (or a proof of concept)
- Affected CLI version (`gravixlayer --version`)

Do not open a public GitHub issue for security vulnerabilities.

We aim to acknowledge reports within a few business days and will coordinate a fix and disclosure timeline with you.

## Release integrity

Official binaries are published on GitHub Releases with per-asset `.sha256` checksums and a release-level `SHA256SUMS` file. The installers verify checksums by default.
