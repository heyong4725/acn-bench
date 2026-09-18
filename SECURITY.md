# Security

- **Reporting.** Use GitHub's private vulnerability reporting for this repository ("Security" → "Report a vulnerability"). Please do not open a public issue for a suspected vulnerability.
- **Secrets.** API keys live in `.env`, which is gitignored, and are read only behind the `real-api` feature. Never commit a key, a provider response containing message content, or a packet capture with payload (CON-21, TRC-42).
- **Measured traces.** Files under `scenarios/measured/` carry timing, size and link-state fields only, with a provenance file. No payload, no identifiers.
- **Dependencies.** `cargo deny check` runs on every PR: advisories, licences, banned crates, and unknown registries or git sources.
