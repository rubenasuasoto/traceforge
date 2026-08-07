# Security policy

## Supported versions

Security fixes target the latest tagged release and the `main` branch.

## Reporting

Please do not open a public issue for a vulnerability involving code execution, data disclosure or a dependency advisory without a fix. Report it privately through GitHub Security Advisories in this repository. Include the affected version, reproduction steps, impact and any suggested mitigation.

TraceForge contains only synthetic fixtures. Do not attach real logs, credentials, tokens, personal data or confidential infrastructure details to a report.

## Design commitments

- No backend, analytics, account system or event upload.
- Browser limits enforced inside the WASM boundary.
- Checksummed, versioned CLI index files.
- Automated Rust and npm dependency auditing in CI.

