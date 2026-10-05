# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |
| < 0.1.0 | No        |

Only the latest release (see Releases page) is supported. Prebuilt
installers (install.sh / install.ps1) always pull the latest release
unless pinned.

## Reporting a Vulnerability

Do not open a public issue for security reports.

1. Go to Security > Advisories > New draft advisory on GitHub.
2. Include: affected version/commit, OS, repro steps or PoC, and impact
   (credential exposure, arbitrary exec, sandbox escape, ...).
3. Expect an initial response within 72 hours.

What happens next:

- We confirm the report and agree on a disclosure timeline (default 90 days).
- We ship a fix and credit you in the release notes (opt-out OK).
- rdsh doctor / rdsh auth output may be requested - redact tokens first.

## Scope Notes

- rdsh delegates the agent loop to the original dsh binary via verbatim exec.
  Bugs in the upstream Harness itself belong upstream; we will help route them.
- The Node dashboard (dashboard/*) binds to loopback by default. Tailscale
  Serve QR URLs are credentials - never paste them into public issues.
