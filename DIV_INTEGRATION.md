# DIV UnifiedService Integration

This document describes the integration of Latvia's DIV UnifiedService for e-invoice delivery through the e-adrese system.

## Overview

The Latvia e-invoice application now supports three delivery methods:

1. **Mock** - For local testing without network calls
2. **DIV UnifiedService** - Latvia's official e-adrese delivery system
3. **Unifiedpost** - PEPPOL access point for cross-border invoices

## DIV UnifiedService Architecture

### Service Endpoint

- **URL**: `https://div.vraa.gov.lv/Vraa.Div.WebService.UnifiedInterface/UnifiedService.svc`
- **Protocol**: SOAP 1.2 with WS-Addressing
- **Authentication**: X509 client certificate

### Key Operations

The DIV UnifiedService provides the following operations for e-invoice delivery:

#### 1. SendMessage
Sends a complete e-invoice wrapped in a DIV Envelope in a single call.

**Inputs**:
- `Envelope` - DIV Envelope XML containing the UBL invoice and metadata
- `AttachmentsInput` - Optional attachment files

**Outputs**:
- `MessageId` - Unique identifier for tracking

#### 2. GetNotificationList
Retrieves status notifications for sent messages.

**Outputs**:
- `Notifications` - List of notification entries with status updates
- `HasMoreData` - Pagination flag

#### 3. GetMessage
Retrieves a sent message for verification.

#### 4. ValidateEAddress
Validates recipient e-adrese identifiers before sending.

## Implementation Status

### ✅ Completed

- Created `DivServiceClient` implementing `AccessPointClient` trait
- Added configuration support for DIV provider
- Updated UI with provider selector dropdown
- Integrated client factory in `main.rs`

### ⚠️ In Progress

- Basic SOAP envelope construction
- DIV Envelope wrapping of UBL invoices
- Status tracking implementation

### 🔲 TODO (Production Readiness)

The current implementation is a **skeleton** that needs significant work before production use:

#### 1. Generate XSD Types

The UnifiedService WSDL references multiple complex XML schemas:
- `http://ivis.eps.gov.lv/XMLSchemas/100001/DIV/v1-0` - DIV Envelope schema
- `http://ivis.eps.gov.lv/XMLSchemas/100001/Person/v1-0` - Person schemas
- `http://ivis.eps.gov.lv/XMLSchemas/100001/Address/v1-1` - Address schemas
- Additional schemas for attachments, signatures, etc.

**Action**: Use `wsdl2rust` or similar tool to generate proper Rust types:

```bash
# Generate Rust bindings from WSDL
wsdl2rust --schemas UnifiedService.xml --output crates/access_point/src/div_types/
```

Or manually define the key types based on the XSDs.

#### 2. Certificate-Based Authentication

DIV UnifiedService requires X509 client certificate authentication. The certificate must:
- Be issued by a trusted CA
- Be registered in the DIV system
- Be used for both TLS handshake AND SOAP message signing

**Action**: Implement proper certificate handling:
```rust
// Configure reqwest to use client certificate
let cert = identity::load_pkcs12(cert_data, cert_password)?;
let client = reqwest::Client::builder()
    .identity(cert)
    .build()?;
```

#### 3. SOAP Security (WSS)

The WSDL specifies WS-Security policy requiring:
- SOAP message signature with X509 token
- Timestamp in the SOAP header
- Proper WS-Addressing headers

**Action**: Use `soap-rs` or similar library to properly construct signed SOAP messages:

```rust
use soap_rs::SoapMessage;

let soap = SoapMessage::new()
    .with_action("http://vraa.gov.lv/div/uui/2011/11/UnifiedServiceInterface/SendMessage")
    .with_timestamp()
    .sign_with_certificate(&cert, &cert_key)?
    .with_body(div_envelope);
```

#### 4. Proper DIV Envelope Construction

The DIV Envelope has complex requirements:

**GeneralMetadata**:
- Title, Date, DocumentKind (must be EINVOICE)
- Authors (sender institution information)
- Description (optional)

**SenderTransportMetadata**:
- SenderE-Address (required)
- SenderRefNumber (unique client reference)
- Recipients (list of RecipientEntry with E-Addresses)
- Priority (high/normal/low)
- DeliveryDeadline (optional)
- NotifySenderOnDelivery (boolean)

**PayloadReference**:
- File metadata (MIME type, size, name)
- ContentReference (CID reference for MTOM attachment)
- DigestMethod and DigestValue (SHA-256 hash of XML)
- Compressed flag

**Action**: Create proper struct definitions with serde serialization:

```rust
#[derive(Debug, Serialize)]
#[serde(rename = "Envelope")]
pub struct DivEnvelope {
    #[serde(rename = "$value")]
    pub sender_document: SenderDocument,
}

#[derive(Debug, Serialize)]
pub struct SenderDocument {
    #[serde(rename = "@Id")]
    pub id: String,
    pub document_metadata: DocumentMetadata,
    pub sender_transport_metadata: SenderTransportMetadata,
}
// ... etc
```

#### 5. UBL Invoice Processing

Extract proper metadata from UBL invoices:
- Invoice ID
- Issue date
- Currency
- Sender/receiver identifiers
- Tax information
- Line items

**Action**: Parse UBL XML using the existing `lat_einv_core::parsing` module and extract required fields.

#### 6. Status Tracking

Implement proper polling using `GetNotificationList`:
- Parse notification types: MessageProcessed, NewMessage, MessageDelivered
- Map DIV statuses to `DeliveryState` enum
- Handle error states and retries

#### 7. Error Handling

DIV service errors are returned in SOAP fault messages:
- HTTP 500 with SOAP Fault body
- Faultcode and Faultstring for details
- May include specific DIV error codes

**Action**: Parse SOAP faults and provide user-friendly error messages.

## Configuration

To use DIV UnifiedService, configure the app as follows:

### Settings UI

1. Open Settings (⚙️ button)
2. Select "DIV UnifiedService (Latvia e-adrese)" from Delivery Method
3. Enter:
   - **Certificate Thumbprint**: Your client certificate's SHA1 or SHA256 thumbprint
   - **Service Address**: `https://div.vraa.gov.lv/Vraa.Div.WebService.UnifiedInterface/UnifiedService.svc`
   - **From E-adrese**: Your sender e-adrese identifier (e.g., `1234567890@vraa.gov.lv`)
   - **From Title**: Your organization name

### Config File

The configuration is stored in:
- macOS: `~/Library/Application Support/lv-einvoice-app/default-config.toml`
- Windows: `%APPDATA%\lv-einvoice-app\config\default-config.toml`

Example:
```toml
[provider]
kind = "div"
base_url = "https://div.vraa.gov.lv/Vraa.Div.WebService.UnifiedInterface/UnifiedService.svc"

[certificate]
thumbprint = "A1B2C3D4E5F6..."

[sender]
from_title = "My Company Ltd"
from_eadrese = "1234567890@vraa.gov.lv"
```

## Certificate Requirements

### Obtaining a Certificate

1. Contact VRAA (Valsts reģionālās attīstības aģentūra) for registration
2. Submit certificate signing request (CSR)
3. Install issued certificate in system certificate store
4. Export certificate and private key for application use

### Certificate Format

- **For TLS**: PKCS#12 (.p12) or PFX format containing certificate and private key
- **For SOAP signing**: Same certificate used for TLS

### Security Storage

Store certificates securely:
- **macOS**: Keychain
- **Windows**: Certificate Store or password-protected files
- **Never** commit certificates to version control

## Testing

### Mock Mode (Current)

The app defaults to mock mode for testing without network access:

1. No certificates required
2. Simulates successful delivery
3. Useful for UI and workflow testing

### DIV Test Environment

If VRAA provides a test/staging environment:

1. Update `base_url` in config to test endpoint
2. Use test certificates
3. Test with small invoices
4. Verify delivery and notifications

### Production

Only use production DIV service after:
- All TODO items completed
- Certificate configured and tested
- Proper error handling verified
- Status tracking confirmed working
- Approved by VRAA

## References

- **VISS e-adrese Guideline**: https://viss.gov.lv/lv/Informacijasistemu-savietotajs/Dokumentacija/Vadlinijas/e-adrese
- **VRAA**: https://www.vraa.gov.lv
- **WSDL**: https://div.vraa.gov.lv/Vraa.Div.WebService.UnifiedInterface/UnifiedService.svc?wsdl
- **DIV Documentation**: Contact VRAA for official documentation

## Migration from Unifiedpost

If switching from Unifiedpost to DIV:

1. **Different protocol**: Unifiedpost uses REST JSON; DIV uses SOAP XML
2. **Different auth**: Unifiedpost uses OAuth2/API key; DIV uses X509 certificates
3. **Different envelope**: UBL vs DIV Envelope + UBL
4. **Different status**: Unifiedpost returns simple states; DIV uses notification system

The `AccessPointClient` trait abstracts these differences, so the core application logic remains unchanged.

## Support

For DIV UnifiedService issues:
- Technical questions: Contact VRAA technical support
- Certificate issues: Certificate authority
- Integration help: This application's development team

For application issues:
- Check logs in console (RUST_LOG=debug)
- Review audit.jsonl for detailed event history
- Test with mock mode first

## License

Proprietary - Internal use only.
