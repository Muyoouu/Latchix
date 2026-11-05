# Security policy

Latchix is pre-release software. Do not run early builds in production.

## Supported versions

| Version | Supported |
|---|---|
| `main` | Development only |
| Releases | None yet |

Security fixes land on `main` first. Release branches are added when the first
release exists.

## Reporting a vulnerability

Do not open a public issue for a vulnerability.

Use the repository's private security advisory feature. If that is unavailable,
contact the maintainer privately through the repository hosting platform.

Include:

- Affected version or commit.
- A description of the issue and its impact.
- Reproduction steps or a proof of concept.
- Whether the issue is already public.
- Any suggested fix or mitigation.

Do not include real credentials, private keys, tenant data, or production
logs. Use synthetic data.

## Response

This is a pre-release project without a paid support contract. The maintainer
will acknowledge a report on a best-effort basis, normally within three
business days, and will triage it within seven business days. There is no bug
bounty and no guaranteed fix deadline.

Please give the maintainer time to investigate and ship a fix before public
disclosure. Credit is offered unless you ask to remain anonymous.

## Scope

In scope:

- SSH and SFTP handling.
- Tenant isolation, ACL enforcement, and path containment.
- Authentication, session handling, and API authorization.
- SQLite metadata handling and migrations.
- Audit integrity.
- Packaging, container, systemd, and release artifacts.
- The optional admin UI.

Out of scope:

- Vulnerabilities in the host operating system, SSH client, or container
  runtime that are not caused by Latchix.
- Denial of service from an operator granting a tenant unlimited resources.
- Social engineering.
- Physical access to a host.

## Known dependency advisories

`RUSTSEC-2023-0071` affects the `rsa` crate used by `russh`. Latchix rejects RSA
host keys at configuration validation. RSA user keys are used for public-key
verification only, so the private-key timing path is not exposed. The advisory
is recorded in `deny.toml` and `.cargo/audit.toml`. This mitigation must be
revisited when the upstream crate provides a constant-time implementation.

## Safe harbor

Good-faith research is welcome. Do not access, modify, or exfiltrate data that
you do not own. Do not test against systems you are not authorized to test.
Stop and report if you encounter real user data.
