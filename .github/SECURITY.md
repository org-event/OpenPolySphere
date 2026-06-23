# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| `main` branch | Yes |
| Latest [release](https://github.com/org-event/Banyan/releases/latest) | Yes |
| Other branches / older releases | Best effort |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Use GitHub **private vulnerability reporting** (preferred):

https://github.com/org-event/Banyan/security/advisories/new

Maintainers can also publish coordinated disclosures via [Security Advisories](https://github.com/org-event/Banyan/security/advisories).

We aim to acknowledge valid reports within **7 days** and share a remediation timeline once confirmed.

## Automated Security

This repository is monitored with:

- Dependabot (dependency CVE alerts and update PRs)
- CodeQL static analysis (Rust, JavaScript, GitHub Actions workflows)
- `cargo audit` / `bun audit` in CI
- Dependency review on pull requests
- Secret scanning and push protection (GitHub)
