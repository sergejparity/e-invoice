# Deployment Guide – Latvian E-Invoice Application

## Packaging for Distribution

### macOS

#### Build the Bundle

```bash
cargo tauri build
```

Output: `src-tauri/target/release/bundle/dmg/Latvian E-Invoice_<version>_<arch>.dmg`

#### Code Signing (Optional but Recommended)

1. **Obtain Apple Developer Certificate**
   - Sign up for [Apple Developer Program](https://developer.apple.com/programs/)
   - Download "Developer ID Application" certificate

2. **Configure Tauri** (`src-tauri/tauri.conf.json`):
   ```json
   {
     "tauri": {
       "bundle": {
         "macOS": {
           "signingIdentity": "Developer ID Application: Your Name (TEAMID)"
         }
       }
     }
   }
   ```

3. **Build with Signing**:
   ```bash
   cargo tauri build
   ```

4. **Notarize** (for Gatekeeper):
   ```bash
   xcrun notarytool submit \
     src-tauri/target/release/bundle/dmg/*.dmg \
     --apple-id your@email.com \
     --team-id TEAMID \
     --password "app-specific-password"
   ```

#### Distribution

- Upload `.dmg` to internal server or distribute directly.
- Users drag app to `/Applications`.

---

### Windows

#### Build the Installer

```bash
cargo tauri build
```

Output: `src-tauri/target/release/bundle/msi/Latvian E-Invoice_<version>_<arch>.msi`

#### Code Signing (Optional but Recommended)

1. **Obtain Code Signing Certificate**
   - Purchase from CA (DigiCert, Sectigo, etc.)
   - Or use internal enterprise certificate

2. **Sign the MSI**:
   ```powershell
   signtool sign /f certificate.pfx /p password /tr http://timestamp.digicert.com /td sha256 /fd sha256 "Latvian E-Invoice_<version>_<arch>.msi"
   ```

#### Distribution

- Upload `.msi` to internal server.
- Users run installer (double-click or `msiexec`).

---

## First-Run Configuration Template

Provide users with a config template file:

**`default-config.toml`**

```toml
[provider]
kind = "mock"  # Change to "unifiedpost" for production
# base_url = "https://api.unifiedpost.com"
# client_id = "your-client-id"
# token_url = "https://api.unifiedpost.com/oauth/token"
```

**Installation Instructions for Users:**

1. macOS: Place config at `~/Library/Application Support/lv-einvoice-app/default-config.toml`
2. Windows: Place config at `%APPDATA%\lv-einvoice-app\config\default-config.toml`

Or let the app create default config on first run.

---

## Production Deployment Checklist

### 1. Obtain Unifiedpost Credentials

- [ ] Sign agreement with Unifiedpost per [VISS e-adrese guidelines](https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese)
- [ ] Receive sandbox credentials (API key or OAuth2 client ID/secret)
- [ ] Test in sandbox environment
- [ ] Receive production credentials

### 2. Configure the Application

- [ ] Edit config file: `kind = "unifiedpost"`, `base_url`, `client_id`
- [ ] Store credentials securely:
  - Option A: Environment variable `UNIFIEDPOST_API_KEY` or `UNIFIEDPOST_CLIENT_SECRET`
  - Option B: OS keychain via `keyring` (see README)

### 3. Test the Build

- [ ] Build app: `cargo tauri build`
- [ ] Install on test machine
- [ ] Run app and verify:
  - Folder picker works
  - XML scan detects invoices
  - Validation runs and shows errors for invalid invoices
  - Send enqueues jobs (check `audit.jsonl` and `.einv_queue`)
  - Jobs transition to "delivered" or "failed"

### 4. Package for Distribution

- [ ] Sign binaries (macOS: Developer ID, Windows: Authenticode)
- [ ] Notarize macOS app (if distributing publicly)
- [ ] Create installation guide with screenshots
- [ ] Document config setup and credential storage

### 5. Deploy

- [ ] Upload installers to internal portal or shared drive
- [ ] Provide users with:
  - Installer (`.dmg` or `.msi`)
  - Config template
  - Credentials setup instructions
  - User manual (see README)

### 6. Monitor & Support

- [ ] Collect `audit.jsonl` logs for compliance audits
- [ ] Monitor job queue database (`.einv_queue`) for stuck jobs
- [ ] Set up log rotation or archival for `audit.jsonl`

---

## Environment-Specific Settings

### Development

```toml
[provider]
kind = "mock"
```

No credentials needed; mock client simulates sending.

### Sandbox (Unifiedpost Test)

```toml
[provider]
kind = "unifiedpost"
base_url = "https://sandbox-api.unifiedpost.com"  # Example sandbox URL
client_id = "sandbox-client-id"
token_url = "https://sandbox-api.unifiedpost.com/oauth/token"
```

Environment variable:
```bash
export UNIFIEDPOST_CLIENT_SECRET="sandbox-secret"
```

### Production

```toml
[provider]
kind = "unifiedpost"
base_url = "https://api.unifiedpost.com"
client_id = "production-client-id"
token_url = "https://api.unifiedpost.com/oauth/token"
```

Environment variable or keychain:
```bash
export UNIFIEDPOST_CLIENT_SECRET="production-secret"
```

---

## Logging & Debugging

### Enable Verbose Logs

macOS/Linux:
```bash
RUST_LOG=debug cargo tauri dev
```

Windows:
```powershell
$env:RUST_LOG="debug"
cargo tauri dev
```

### Log Locations

- **Structured logs**: stdout (in dev mode) or system logs (in production)
- **Audit log**: `audit.jsonl` in app working directory
- **Job queue**: `.einv_queue/` directory (sled database)

### Common Issues

**Jobs not progressing:**
- Check `RUST_LOG=lat_einv_queue=debug` for queue worker errors.
- Verify Unifiedpost credentials are correct.
- Check network connectivity.

**Validation errors:**
- Review error messages in UI.
- Inspect XML against [UBL 2.1 EN16931 specification](https://docs.peppol.eu/poacc/billing/3.0/).

---

## Rollback Plan

If production deployment fails:

1. **Revert to Mock Mode**: Change config `kind = "mock"` and restart app.
2. **Re-examine Jobs**: Query `.einv_queue` database to check stuck jobs.
3. **Review Audit Log**: `audit.jsonl` tracks all send attempts.
4. **Contact Support**: Unifiedpost support or VISS e-adrese helpdesk.

---

## Security Best Practices

- **Never commit credentials** to version control.
- **Use OS keychain** for production secrets.
- **Rotate API keys** periodically.
- **Restrict access** to config files and audit logs.
- **Encrypt audit logs** if storing on shared drives.

---

## Support & Resources

- **VISS e-adrese**: https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese
- **Unifiedpost**: https://www.unifiedpost.com/contact
- **PEPPOL BIS 3.0**: https://docs.peppol.eu/poacc/billing/3.0/
- **Tauri Documentation**: https://tauri.app/v1/guides/


