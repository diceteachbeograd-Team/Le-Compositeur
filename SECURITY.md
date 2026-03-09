# Security Policy

## Supported Versions
This project is in early development.

Security fixes are currently provided for:
- `main` branch (latest state)

Older snapshots, forks, or unpublished local branches are not guaranteed to receive security fixes.

## Reporting a Vulnerability
Please do not open public issues for sensitive security problems.

Until a dedicated security email is created, report vulnerabilities by opening a private communication channel with maintainers (preferred), or if unavailable:
1. Open a minimal public issue without exploit details.
2. Ask maintainers for a private follow-up path.

When reporting, include:
- affected component (`wc-cli`, `wc-core`, `wc-render`)
- impact summary
- reproduction steps
- proof-of-concept (if safe)
- proposed mitigation (if known)

## Network Access & Widgets
- `Weather` and `News` widgets can require internet access to fetch external data.
- On a fresh install, both widgets are disabled by default.
- Enabling them may send requests to third-party endpoints configured in presets or custom URLs.
- For privacy-sensitive setups, keep these widgets disabled and use local-only image/quote sources.

## Response Targets
- Initial triage response target: within 7 days
- Status update target after triage: within 14 days
- Fix delivery target: depends on severity and scope

## Disclosure Process
1. Report received and triaged.
2. Maintainers confirm impact and severity.
3. Fix is developed and tested.
4. Security release/changelog entry is published.
5. Public disclosure follows after a fix is available.
