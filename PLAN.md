# Latvia e‑Invoice GUI via PEPPOL (Rust + Tauri)

Source of truth for Latvia guidance and PEPPOL integration path:

- Unifiedpost is the PEPPOL Access Point for e‑adrese integration; institutions must conclude an agreement before sending/receiving PEPPOL e‑invoices. [VISS e‑adrese guideline](https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese)
- e‑invoices are structured XML; for e‑adrese messages they are attached with DocumentKindCode EINVOICE, but for our app we’ll send via PEPPOL AP (Unifiedpost) per your choice. [VISS e‑adrese guideline](https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese)

## Scope

- Desktop GUI for macOS and Windows.
- User selects a local folder containing UBL 2.1 EN16931 (PEPPOL BIS 3.0) XML invoices.
- Validate invoices (XSD + basic EN16931 rules), preview errors, then send via PEPPOL AP (Unifiedpost). No signing required.
- Track send status, retries, and provide an audit log.

## Key Tech Choices

- GUI: Tauri (Rust backend + lightweight web UI) for macOS/Windows.
- Rust crates: quick-xml/roxmltree for parsing, reqwest for HTTP, serde for config, notify for file watching, keyring for OS secrets.
- Optional validation: libxml2 (via rust bindings) for XSD; lightweight EN16931 rule checks implemented in Rust; schematron (future).

## High‑Level Architecture

- app/ (Tauri frontend): UI to pick folder, list invoices, show validation/send status.
- src-tauri/
  - src/main.rs: Tauri bootstrap and command wiring.
  - src/commands.rs: IPC commands (pick folder, scan, validate, send, status).
- crates/core/
  - parsing.rs: load + parse UBL XML, extract identifiers.
  - validation/{xsd.rs, rules.rs}: XSD + basic EN16931 checks.
  - models.rs: invoice metadata, send job, status.
- crates/access_point/
  - trait AccessPointClient { submit, status, delivery_feedback }
  - unifiedpost.rs: Unifiedpost implementation (env/config‑driven endpoints, OAuth2/API‑key).
  - mock.rs: local stub for dev without credentials.
- crates/queue/
  - queue.rs: persistent job queue (sled/sqlite), retries, idempotency keys.
  - worker.rs: async sender with rate limiting and backoff.
- crates/config/
  - config.rs: app config; provider credentials via OS keychain.
- logs/: rotating logs, audit trail (jsonl).

## Core Flows

1) Select folder → scan for *.xml → parse/validate → display results.

2) Configure provider (Unifiedpost) → store credentials securely.

3) Enqueue valid invoices → background worker sends via AP → show per‑invoice status.

4) Auto‑watch folder (optional) to enqueue new XML files.

## Provider Integration (Unifiedpost)

- Abstraction via AccessPointClient trait; start with Mock client.
- Real Unifiedpost client supports:
  - auth: OAuth2 client credentials or API key (configurable)
  - submit(invoice_xml, senderId, receiverId, profile) → returns transmission id
  - poll status by transmission id; handle delivery receipts
- All network activity behind a rate limiter and resilient retry policy.

## Files to Create (essential)

- src-tauri/src/main.rs
- src-tauri/src/commands.rs
- app/src/pages/Home.tsx (or Svelte equivalent)
- crates/core/src/{parsing.rs,validation/xsd.rs,validation/rules.rs,models.rs,lib.rs}
- crates/access_point/src/{lib.rs,unifiedpost.rs,mock.rs}
- crates/queue/src/{queue.rs,worker.rs,lib.rs}
- crates/config/src/{config.rs,lib.rs}

## Example Trait (essential snippet)

```rust
pub trait AccessPointClient: Send + Sync {
    fn submit(&self, xml: &str, sender: &str, receiver: &str, profile: &str) -> anyhow::Result<String>; // returns transmission id
    fn status(&self, transmission_id: &str) -> anyhow::Result<DeliveryStatus>;
}
```

## Security & Ops

- Store credentials in OS keychain; redact secrets in logs.
- Idempotency per invoice hash; prevent duplicate sends.
- Offline‑tolerant queue; resume on restart.

## Packaging

- Tauri bundling for .app (macOS) and .msi/.exe (Windows).
- App config file + migration safe defaults.

## Deliverables

- Working GUI app with mock AP.
- Unifiedpost client wired behind feature flag; can switch to real once credentials are provided.
- Validation and status UI, persistent queue, logs.

## Notes

- Per VISS guidance, PEPPOL via Unifiedpost requires a contract before production traffic. [VISS e‑adrese guideline](https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese)

### To-dos

- [x] Scaffold Tauri app (Rust backend + React/Svelte UI)
- [x] Implement folder picker, XML listing, validation results UI
- [x] Add UBL XML parsing and XSD/basic EN16931 validation
- [x] Define AccessPointClient trait and wire IPC commands
- [x] Implement mock access point client for local dev
- [x] Build persistent job queue and async sender with retries
- [x] Implement Unifiedpost client (auth, submit, status)
- [x] Add config management and OS keychain secrets storage
- [x] Add structured logs and audit trail
- [x] Bundle installers for macOS and Windows with config templates
