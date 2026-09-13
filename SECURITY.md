# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |
| Older versions | No |

Security fixes target the latest release.

## Reporting a vulnerability

Please report vulnerabilities privately through [GitHub Security Advisories](https://github.com/suiflex/kurir/security/advisories/new). Do not open a public issue for an unpatched vulnerability.

Include the affected version, operating system, harness, reproduction steps, and impact. Remove secrets from reports and configuration samples.

## In scope

- Writes escaping the requested configuration path.
- Symlink or path traversal vulnerabilities during config updates.
- Secret disclosure through previews, errors, backups, or logs.
- `--dry-run` invoking a harness command or writing a file.
- Unsafe parsing or command argument injection in delegated harness adapters.
- Downloading an unexpected native binary through the npm or install scripts.
- Incorrect checksums or release asset substitution.
- Cross-user or cross-project configuration writes.

## Expected behavior

Kurir intentionally launches user-selected MCP commands when registration is requested. Reports that require prior local control of the command, environment, or machine are generally outside scope unless they demonstrate an additional privilege boundary bypass.
