use super::{
    div_types::{compute_sha256_base64, DivEnvelope},
    AccessPointClient, DeliveryState, DeliveryStatus,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use lat_einv_core::parsing::parse_ubl_invoice;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

/// SOAP response wrapper for DIV service
#[derive(Debug, Deserialize)]
#[serde(rename = "Envelope")]
struct SoapEnvelope {
    #[serde(rename = "Body")]
    body: SoapBody,
}

#[derive(Debug, Deserialize)]
struct SoapBody {
    #[serde(rename = "$value")]
    content: String,
}

/// DIV SendMessage response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SendMessageOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    message_id: Option<String>,
}

/// DIV GetNotificationList response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NotificationListOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    notifications: Option<NotificationArray>,
    has_more_data: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NotificationArray {
    #[serde(rename = "Notification")]
    notification: Vec<Notification>,
}

/// Individual notification entry
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Notification {
    id: Option<i64>,
    #[serde(rename = "Type")]
    notification_type: Option<NotificationType>,
    created_on: Option<String>,
    message_id: Option<String>,
    message_status: Option<MessageStatus>,
    status_code: Option<String>,
    status_text: Option<String>,
}

#[derive(Debug, Deserialize)]
enum NotificationType {
    MessageProcessed,
    NewMessage,
    MessageDelivered,
}

#[derive(Debug, Deserialize)]
enum MessageStatus {
    #[serde(rename = "New")]
    New,
    #[serde(rename = "Sent")]
    Sent,
    #[serde(rename = "Rejected")]
    Rejected,
    #[serde(rename = "Accepted")]
    Accepted,
    #[serde(rename = "DeliveryDelayed")]
    DeliveryDelayed,
    #[serde(rename = "RecipientAccepted")]
    RecipientAccepted,
    #[serde(rename = "RecipientRejected")]
    RecipientRejected,
}

/// DIV UnifiedService client for Latvia e-adrese integration
/// 
/// This client implements the DIV UnifiedService API as defined in:
/// https://div.vraa.gov.lv/Vraa.Div.WebService.UnifiedInterface/UnifiedService.svc
/// 
/// The service uses SOAP/WS-Addressing with X509 certificate authentication.
/// E-invoices are sent via the SendMessage operation, which wraps UBL invoices
/// in a DIV Envelope structure for e-adrese delivery.
#[derive(Clone)]
pub struct DivServiceClient {
    /// Base URL of the DIV UnifiedService endpoint
    pub base_url: String,
    /// Client certificate thumbprint for X509 authentication
    pub cert_thumbprint: String,
    /// Sender's e-adrese identifier
    pub sender_eaddress: String,
    /// HTTP client configured for SOAP requests
    http_client: reqwest::Client,
}

impl DivServiceClient {
    /// Create a new DIV UnifiedService client
    ///
    /// # Arguments
    /// * `base_url` - Full URL to UnifiedService.svc endpoint
    /// * `cert_thumbprint` - SHA1 or SHA256 thumbprint of client certificate
    /// * `sender_eaddress` - Sender's e-adrese (format: AuthorityID@domain or similar)
    ///
    /// # Example
    /// ```
    /// let client = DivServiceClient::new(
    ///     "https://div.vraa.gov.lv/.../UnifiedService.svc".to_string(),
    ///     "A1B2C3D4E5F6...".to_string(),
    ///     "1234567890@vraa.gov.lv".to_string(),
    /// );
    /// ```
    pub fn new(base_url: String, cert_thumbprint: String, sender_eaddress: String) -> Arc<Self> {
        // Build HTTP client with longer timeout for SOAP requests
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Arc::new(Self {
            base_url,
            cert_thumbprint,
            sender_eaddress,
            http_client,
        })
    }

    /// Build the SOAP envelope for SendMessage request
    ///
    /// The DIV UnifiedService uses SOAP 1.2 with WS-Addressing and WS-Security.
    /// According to the WSDL policy, it requires:
    /// - X509 certificate for authentication
    /// - SOAP message signature
    /// - Timestamp in header
    /// - WS-Addressing headers
    ///
    /// ⚠️ CURRENT LIMITATION: This implementation doesn't yet sign the SOAP message.
    /// For production use, you would need to:
    /// 1. Add WS-Security signing using a library like `soap-rs` or manually with OpenSSL
    /// 2. Include Timestamp element in SOAP header
    /// 3. Sign the SOAP body with the X509 certificate
    fn build_soap_envelope(&self, envelope_xml: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" xmlns:a="http://www.w3.org/2005/08/addressing">
    <s:Header>
        <a:Action s:mustUnderstand="1">http://vraa.gov.lv/div/uui/2011/11/UnifiedServiceInterface/SendMessage</a:Action>
        <a:To s:mustUnderstand="1">{}</a:To>
    </s:Header>
    <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
        <SendMessageInput xmlns="http://vraa.gov.lv/xmlschemas/div/uui/2011/11">
            {}
        </SendMessageInput>
    </s:Body>
</s:Envelope>"#,
            self.base_url,
            envelope_xml
        )
    }

    /// Build a DIV Envelope for an e-invoice
    fn build_div_envelope(
        &self,
        ubl_xml: &str,
        recipient_eaddress: &str,
        sender_org_name: &str,
    ) -> Result<DivEnvelope> {
        // Parse UBL to extract metadata
        let invoice = parse_ubl_invoice(ubl_xml)
            .context("Failed to parse UBL invoice for DIV envelope")?;

        // Compute SHA-256 digest of the UBL XML
        let digest = compute_sha256_base64(ubl_xml.as_bytes());

        // Create DIV Envelope using the structured types
        let envelope = DivEnvelope::new(
            format!("E-invoice: {}", invoice.invoice_number),
            invoice.issue_date,
            self.sender_eaddress.clone(),
            format!("ref-{}", uuid::Uuid::new_v4()),
            recipient_eaddress.to_string(),
            sender_org_name.to_string(),
            "invoice.xml".to_string(),
            "application/xml".to_string(),
            ubl_xml.len() as u64,
            digest,
        );

        Ok(envelope)
    }

    /// Get authentication headers for the SOAP request
    ///
    /// DIV UnifiedService requires X509 certificate-based authentication.
    /// The actual certificate signing is typically handled at the TLS layer
    /// via client certificate authentication. This method prepares additional
    /// headers that might be needed.
    fn get_auth_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Content-Type", "application/soap+xml; charset=utf-8".to_string()),
            ("SOAPAction", "http://vraa.gov.lv/div/uui/2011/11/UnifiedServiceInterface/SendMessage".to_string()),
        ]
    }

    /// Build SOAP request for GetNotificationList
    fn build_notification_list_soap(&self, max_results: i32) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" xmlns:a="http://www.w3.org/2005/08/addressing">
    <s:Header>
        <a:Action s:mustUnderstand="1">http://vraa.gov.lv/div/uui/2011/11/UnifiedServiceInterface/GetNotificationList</a:Action>
        <a:To s:mustUnderstand="1">{}</a:To>
    </s:Header>
    <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
        <GetNotificationListInput xmlns="http://vraa.gov.lv/xmlschemas/div/uui/2011/11">
            <MaxResultCount>{}</MaxResultCount>
        </GetNotificationListInput>
    </s:Body>
</s:Envelope>"#,
            self.base_url,
            max_results
        )
    }

    /// Map DIV MessageStatus to our DeliveryState
    fn map_status(div_status: &MessageStatus) -> DeliveryState {
        match div_status {
            MessageStatus::New | MessageStatus::Sent | MessageStatus::DeliveryDelayed => {
                DeliveryState::InFlight
            }
            MessageStatus::Accepted | MessageStatus::RecipientAccepted => DeliveryState::Delivered,
            MessageStatus::Rejected | MessageStatus::RecipientRejected => DeliveryState::Failed,
        }
    }
}

#[async_trait]
impl AccessPointClient for DivServiceClient {
    /// Submit an e-invoice to the DIV UnifiedService
    ///
    /// This wraps the UBL invoice in a DIV Envelope and sends it via SOAP.
    /// The service handles routing to the recipient through the e-adrese system.
    ///
    /// # Returns
    /// A message ID that can be used to query delivery status
    async fn submit(
        &self,
        xml: &str,
        sender: &str,
        receiver: &str,
        profile: &str,
    ) -> Result<String> {
        // Parse UBL invoice to get supplier name
        let invoice = parse_ubl_invoice(xml)
            .context("Failed to parse UBL invoice")?;
        
        // Use supplier name from UBL, or fallback to a generic value
        let sender_org_name = if !invoice.supplier_name.is_empty() {
            invoice.supplier_name.clone()
        } else {
            "E-Invoice Sender".to_string()
        };

        // Build DIV Envelope using structured types
        let div_envelope = self.build_div_envelope(xml, receiver, &sender_org_name)?;
        
        // Get SenderRefNumber from the envelope for tracking
        let invoice_id = div_envelope.sender_document.sender_transport_metadata.sender_ref_number.clone();

        // Serialize DIV Envelope to XML
        let div_envelope_xml = div_envelope.to_xml();

        // Build SOAP envelope
        let soap_body = self.build_soap_envelope(&div_envelope_xml);

        // Send SOAP request
        let response = self
            .http_client
            .post(&self.base_url)
            .headers({
                let mut headers = reqwest::header::HeaderMap::new();
                for (key, value) in self.get_auth_headers() {
                    headers.insert(
                        reqwest::header::HeaderName::from_static(key),
                        value.parse().unwrap(),
                    );
                }
                headers
            })
            .body(soap_body)
            .send()
            .await
            .context("Failed to send SOAP request to DIV UnifiedService")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("DIV UnifiedService submit failed: {} - {}", status, body);
        }

        let response_body = response.text().await
            .context("Failed to read DIV UnifiedService response")?;

        // Parse SOAP response to extract message ID
        // For now, return a placeholder. In production, properly parse the SOAP/XML response.
        tracing::info!(
            message_id = %invoice_id,
            "Invoice submitted to DIV UnifiedService"
        );

        Ok(invoice_id)
    }

    /// Query the delivery status of an e-invoice
    ///
    /// DIV UnifiedService provides status tracking via the GetNotificationList operation.
    /// This method polls for notifications and maps DIV statuses to our DeliveryState enum.
    async fn status(&self, message_id: &str) -> Result<DeliveryStatus> {
        // Build SOAP request for GetNotificationList
        let soap_request = self.build_notification_list_soap(100);

        // Send SOAP request
        let response = self
            .http_client
            .post(&self.base_url)
            .headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::HeaderName::from_static("Content-Type"),
                    "application/soap+xml; charset=utf-8".parse().unwrap(),
                );
                headers.insert(
                    reqwest::header::HeaderName::from_static("SOAPAction"),
                    "http://vraa.gov.lv/div/uui/2011/11/UnifiedServiceInterface/GetNotificationList"
                        .parse()
                        .unwrap(),
                );
                headers
            })
            .body(soap_request)
            .send()
            .await
            .context("Failed to query DIV UnifiedService notifications")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("DIV notification query failed: {} - {}", status, body);
        }

        let response_body = response
            .text()
            .await
            .context("Failed to read DIV notification response")?;

        // Parse SOAP response
        // For simplicity, we'll search the raw XML for our message ID
        // In production, you'd properly parse the full SOAP/XML structure
        
        tracing::debug!(
            message_id = %message_id,
            "Polled DIV UnifiedService notifications"
        );

        // If we can't find the message, assume it's still in flight
        // TODO: Properly parse SOAP response and find matching notification
        Ok(DeliveryStatus {
            transmission_id: message_id.to_string(),
            state: DeliveryState::InFlight,
            message: Some("Notification parsing not yet fully implemented".to_string()),
        })
    }
}
