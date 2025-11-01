# Latvian E-Invoice Application

Cross-platform desktop application for sending e-invoices in Latvia with support for multiple delivery methods.

## Features

- **GUI Application** – Tauri-based native app for macOS and Windows
- **UBL 2.1 EN16931 Validation** – Parse and validate PEPPOL BIS 3.0 invoices
- **Multiple Delivery Methods**:
  - **DIV UnifiedService** – Latvia's official e-adrese system (SOAP/X509)
  - **Unifiedpost** – PEPPOL access point for cross-border invoices
  - **Mock** – Local testing mode
- **Persistent Queue** – Background sender with retries and delivery tracking
- **Audit Trail** – JSON lines audit log for compliance
- **Secure Credentials** – OS keychain integration for API keys and certificates

## Quick Start

### Prerequisites

- Rust (stable) and Cargo
- Tauri CLI: `cargo install tauri-cli@^1`
- macOS: Xcode Command Line Tools
- Windows: Visual Studio Build Tools + Windows SDK

### Run in Development

```bash
cargo tauri dev
```

### Build Production Bundle

```bash
cargo tauri build
```

Outputs:
- macOS: `src-tauri/target/release/bundle/dmg/Latvian E-Invoice_<version>_<arch>.dmg`
- Windows: `src-tauri/target/release/bundle/msi/Latvian E-Invoice_<version>_<arch>.msi`

## Configuration

The app uses a config file stored by `confy`:
- macOS: `~/Library/Application Support/lv-einvoice-app/default-config.toml`
- Windows: `%APPDATA%\lv-einvoice-app\config\default-config.toml`

### Default Configuration (Mock Mode)

By default, the app runs with a **mock access point** that simulates sending without network calls.

```toml
[provider]
kind = "mock"
```

### DIV UnifiedService Configuration (Latvia e-adrese)

To use Latvia's official DIV UnifiedService:

**⚠️ Status**: Currently in development. See `DIV_INTEGRATION.md` for complete details.

1. **Edit the config file**:
   ```toml
   [provider]
   kind = "div"
   base_url = "https://div.vraa.gov.lv/Vraa.Div.WebService.UnifiedInterface/UnifiedService.svc"
   
   [certificate]
   thumbprint = "your-cert-thumbprint"
   
   [sender]
   from_title = "Your Company Ltd"
   from_eadrese = "your-identifier@vraa.gov.lv"
   ```

2. **Obtain certificates** from VRAA and store securely.

### Unifiedpost Configuration (PEPPOL)

To enable real PEPPOL sending via Unifiedpost:

1. **Edit the config file**:
   ```toml
   [provider]
   kind = "unifiedpost"
   base_url = "https://api.unifiedpost.com"
   client_id = "your-client-id"
   token_url = "https://api.unifiedpost.com/oauth/token"
   ```

2. **Store credentials securely** using environment variables OR OS keychain.

#### Option A: Environment Variables

```bash
export UNIFIEDPOST_API_KEY="your-api-key"
# OR for OAuth2:
export UNIFIEDPOST_CLIENT_SECRET="your-client-secret"
```

#### Option B: OS Keychain

Use a Rust script or manually add to keychain:

```rust
use keyring::Entry;

fn main() {
    let entry = Entry::new("lv.einvoice.credentials", "unifiedpost_api_key").unwrap();
    entry.set_password("your-api-key").unwrap();
    // OR for OAuth2:
    let entry_secret = Entry::new("lv.einvoice.credentials", "unifiedpost_client_secret").unwrap();
    entry_secret.set_password("your-client-secret").unwrap();
}
```

## Usage

1. **Pick a Folder** – Click "Pick folder…" and select a directory containing UBL XML invoices.
2. **Scan XML** – Lists all `.xml` files in the selected folder.
3. **Validate** – Checks invoices against EN16931 mandatory fields (invoice number, issue date, currency, seller, buyer, amounts).
4. **Send** – Enqueues valid invoices to the background sender.
5. **Monitor Jobs** – Watch job status table for delivery updates (every 2 seconds auto-refresh).

## Audit Log

All invoice send events are logged to `audit.jsonl` in JSON Lines format:

```json
{"timestamp":"2025-01-29T12:34:56Z","event_type":"job_enqueued","job_id":"abc123","invoice_hash":"sha256...","state":"queued","sender":"LV:123456","receiver":"LV:789012"}
{"timestamp":"2025-01-29T12:35:01Z","event_type":"invoice_submitted","job_id":"abc123","transmission_id":"unp-xyz789","state":"sent"}
{"timestamp":"2025-01-29T12:35:02Z","event_type":"delivery_status_updated","job_id":"abc123","transmission_id":"unp-xyz789","state":"delivered"}
```

## Architecture

- **Tauri Backend** (`src-tauri/`) – Rust app handling IPC commands, queue, validation.
- **Static UI** (`ui/`) – Vanilla HTML/JS frontend for folder picker, invoice list, status table.
- **Core Crates**:
  - `crates/core` – UBL parsing, EN16931 validation.
  - `crates/access_point` – AccessPointClient trait, Mock + Unifiedpost + DIV implementations.
  - `crates/queue` – Persistent sled-backed job queue with async sender.
  - `crates/config` – App config and OS keychain integration.

## Delivery Methods

### DIV UnifiedService (Latvia e-adrese)

Latvia's official e-adrese system for domestic e-invoice delivery via SOAP/X509.

**⚠️ Status**: Currently in development. See `DIV_INTEGRATION.md` for implementation details.

**Requirements**:
- Client X509 certificate registered with VRAA
- Agreement with VRAA for service access

### Unifiedpost (PEPPOL)

Per VISS e-adrese guidelines, Unifiedpost is the PEPPOL Access Point for Latvia's e-adrese integration. Institutions must conclude an agreement with Unifiedpost before production use.

**References:**
- [VISS e-adrese guideline](https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese)
- [Unifiedpost PEPPOL](https://www.unifiedpost.com/)
- [VRAA](https://www.vraa.gov.lv)

## Development

### Project Structure

```
e-invoice/
├── Cargo.toml              # Workspace manifest
├── src-tauri/              # Tauri app
│   ├── src/
│   │   ├── main.rs         # App bootstrap, client factory
│   │   └── commands.rs     # IPC commands (scan, validate, enqueue, list_status)
│   ├── tauri.conf.json     # Tauri config
│   └── Cargo.toml
├── ui/                     # Static HTML/JS UI
│   ├── index.html
│   └── main.js
└── crates/
    ├── core/               # Parsing & validation
    ├── access_point/       # Mock, Unifiedpost, DIV clients
    ├── queue/              # Job queue + audit log
    └── config/             # Config + keychain
```

### Testing

```bash
# Check all crates
cargo check

# Format
cargo fmt

# Run tests (when added)
cargo test
```

## Troubleshooting

### "DIV UnifiedService not configured"
- Ensure `provider.kind = "div"` in config.
- Set `base_url`, `certificate.thumbprint`, and `sender.from_eadrese`.
- Verify certificate is properly installed and registered with VRAA.

### "Unifiedpost client not configured"
- Ensure `provider.kind = "unifiedpost"` in config.
- Set `base_url` and either `UNIFIEDPOST_API_KEY` or `client_id` + `UNIFIEDPOST_CLIENT_SECRET`.

### "Invoice validation failed"
- Check XML structure matches UBL 2.1 EN16931 (PEPPOL BIS 3.0).
- Mandatory fields: `<ID>`, `<IssueDate>`, `<DocumentCurrencyCode>`, `<AccountingSupplierParty>`, `<AccountingCustomerParty>`, `<LegalMonetaryTotal><PayableAmount>`.

### Jobs stuck in "queued"
- Check logs: `RUST_LOG=debug cargo tauri dev`.
- Verify network access if using Unifiedpost.

## License

Proprietary – for internal use.

## Contact

For Unifiedpost onboarding or support, contact:
- [Unifiedpost Support](https://www.unifiedpost.com/contact)
- VISS e-adrese support: atbalsts@vdaa.gov.lv


