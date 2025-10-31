# Quick Start Guide

## Installation

### macOS

1. Install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. Install Tauri CLI:
   ```bash
   cargo install tauri-cli@^1
   ```

3. Run the app:
   ```bash
   cargo tauri dev
   ```

### Windows

1. Install Rust: Download from [rustup.rs](https://rustup.rs/)

2. Install Build Tools:
   - **Download** [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
   - **Run installer** and select **"Desktop development with C++"** workload
   - This includes the necessary C++ compilers and Windows SDK
   - Click **"Install"** and wait for completion
   
   > **Note:** If Windows SDK wasn't included, download separately from [Windows SDK page](https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/)

3. **Verify Installation:**
   - Open a **new** terminal/powershell window
   - Run: `cl` - should show compiler info (not "command not found")
   - Run: `rustc --version` - should show Rust version

4. Install Tauri CLI:
   ```powershell
   cargo install tauri-cli@^1
   ```

5. Run the app:
   ```powershell
   cargo tauri dev
   ```
   
   > **Note:** On first run, this will compile all dependencies and may take several minutes

## Basic Usage (Mock Mode)

The app starts in **mock mode** by default – no credentials needed!

1. **Launch** the app (`cargo tauri dev`)
2. **Pick folder** containing XML invoice files
3. **Scan XML** to list all `.xml` files
4. **Validate** to check EN16931 compliance
5. **Send** to enqueue invoices (mock simulates delivery)
6. **Monitor** the Jobs table – invoices will show as "delivered"

## Switching to Production (Unifiedpost)

### Step 1: Get Credentials

Contact Unifiedpost to obtain:
- API Key, OR
- OAuth2 Client ID + Client Secret

### Step 2: Configure

Edit config file:
- macOS: `~/Library/Application Support/lv-einvoice-app/default-config.toml`
- Windows: `%APPDATA%\lv-einvoice-app\config\default-config.toml`

```toml
[provider]
kind = "unifiedpost"
base_url = "https://api.unifiedpost.com"
client_id = "your-client-id"
token_url = "https://api.unifiedpost.com/oauth/token"
```

### Step 3: Store Secret

Set environment variable:

macOS/Linux:
```bash
export UNIFIEDPOST_CLIENT_SECRET="your-secret"
cargo tauri dev
```

Windows:
```powershell
$env:UNIFIEDPOST_CLIENT_SECRET="your-secret"
cargo tauri dev
```

Or use OS keychain (see README.md for details).

### Step 4: Test

Send a test invoice and verify it appears in your Unifiedpost dashboard.

## Building for Distribution

### macOS
```bash
cargo tauri build
# Output: src-tauri/target/release/bundle/dmg/*.dmg
```

### Windows
```bash
cargo tauri build
# Output: src-tauri/target/release/bundle/msi/*.msi
```

## Troubleshooting

**App won't start:**
- Check Rust is installed: `rustc --version`
- Check Tauri CLI: `cargo tauri --version`

**Validation fails:**
- Ensure XML is UBL 2.1 EN16931 format
- Check mandatory fields: invoice number, issue date, currency, seller, buyer

**Jobs stuck:**
- Enable debug logs: `RUST_LOG=debug cargo tauri dev`
- Verify Unifiedpost credentials
- Check network connectivity

## Next Steps

- Read [README.md](README.md) for detailed architecture
- Read [DEPLOYMENT.md](DEPLOYMENT.md) for production setup
- Review `audit.jsonl` for send history
- Inspect `.einv_queue/` for job queue state

## Support

- VISS e-adrese: https://viss.gov.lv/lv/Informacijai/Dokumentacija/Vadlinijas/e-adrese
- Unifiedpost: https://www.unifiedpost.com/contact
- PEPPOL BIS 3.0: https://docs.peppol.eu/poacc/billing/3.0/


