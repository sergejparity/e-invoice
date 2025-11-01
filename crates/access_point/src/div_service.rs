use super::{AccessPointClient, DeliveryState, DeliveryStatus};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

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
    /// The DIV UnifiedService uses SOAP 1.2 with WS-Addressing.
    /// E-invoices are embedded in the DIV Envelope structure within the SOAP body.
    fn build_soap_envelope(&self, envelope_xml: &str) -> String {
        // Note: This is a simplified SOAP envelope. In production, you should:
        // 1. Use the WSDL to generate proper Rust types from the XSDs
        // 2. Include proper WS-Addressing headers
        // 3. Sign the SOAP message with the X509 certificate
        // 4. Use proper namespaces and formatting
        
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" xmlns:a="http://www.w3.org/2005/08/addressing">
    <s:Header>
        <a:Action s:mustUnderstand="1">http://vraa.gov.lv/div/uui/2011/11/UnifiedServiceInterface/SendMessage</a:Action>
        <a:To s:mustUnderstand="1">{}</a:To>
    </s:Header>
    <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
        <SendMessageInput xmlns="http://vraa.gov.lv/xmlschemas/div/uui/2011/11">
            <Envelope xmlns="http://ivis.eps.gov.lv/XMLSchemas/100001/DIV/v1-0">
                {}
            </Envelope>
        </SendMessageInput>
    </s:Body>
</s:Envelope>"#,
            self.base_url,
            envelope_xml
        )
    }

    /// Build a DIV Envelope XML for an e-invoice
    ///
    /// The DIV Envelope wraps UBL invoices with metadata needed for e-adrese delivery:
    /// - GeneralMetadata: title, date, document kind (EINVOICE), authors, etc.
    /// - SenderTransportMetadata: sender e-adrese, recipients, priority, etc.
    /// - PayloadReference: references to attached files (the UBL invoice itself)
    ///
    /// This is a simplified implementation. A full implementation should use generated
    /// XSD types from the DIV schema definitions.
    fn build_div_envelope(
        &self,
        ubl_xml: &str,
        recipient_eaddress: &str,
        invoice_id: &str,
        invoice_date: &str,
    ) -> Result<String> {
        // For now, we'll create a basic DIV envelope structure
        // In production, you should properly serialize this using derived structs from the XSD
        
        Ok(format!(
            r#"<SenderDocument Id="SenderSection">
    <DocumentMetadata>
        <GeneralMetadata>
            <Title>E-invoice</Title>
            <Date>{}</Date>
            <DocumentKind>
                <DocumentKindCode>EINVOICE</DocumentKindCode>
                <DocumentKindVersion>1.0</DocumentKindVersion>
                <DocumentKindName>E-invoice</DocumentKindName>
            </DocumentKind>
            <Authors>
                <AuthorEntry>
                    <Institution>
                        <Title>Sender Company</Title>
                    </Institution>
                </AuthorEntry>
            </Authors>
        </GeneralMetadata>
        <PayloadReference>
            <File>
                <MimeType>application/xml</MimeType>
                <Size>{}</Size>
                <Name>invoice.xml</Name>
                <Content>
                    <ContentReference>cid:invoice-content</ContentReference>
                    <DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
                    <DigestValue>TODO_CALCULATE_HASH</DigestValue>
                </Content>
                <Compressed>false</Compressed>
            </File>
        </PayloadReference>
    </DocumentMetadata>
    <SenderTransportMetadata>
        <SenderE-Address>{}</SenderE-Address>
        <SenderRefNumber>{}</SenderRefNumber>
        <Recipients>
            <RecipientEntry>
                <RecipientE-Address>{}</RecipientE-Address>
            </RecipientEntry>
        </Recipients>
        <NotifySenderOnDelivery>true</NotifySenderOnDelivery>
        <Priority>normal</Priority>
    </SenderTransportMetadata>
</SenderDocument>"#,
            invoice_date,
            ubl_xml.len(),
            self.sender_eaddress,
            invoice_id,
            recipient_eaddress
        ))
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
        // Extract invoice metadata from UBL XML
        // For now, we'll use basic parsing. In production, use a proper UBL parser.
        let invoice_id = format!("inv-{}", uuid::Uuid::new_v4());
        let invoice_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Build DIV Envelope
        let div_envelope = self.build_div_envelope(
            xml,
            receiver,
            &invoice_id,
            &invoice_date,
        )?;

        // Build SOAP envelope
        let soap_body = self.build_soap_envelope(&div_envelope);

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
    /// This method maps DIV statuses to our DeliveryState enum.
    async fn status(&self, message_id: &str) -> Result<DeliveryStatus> {
        // TODO: Implement proper status query using GetNotificationList
        // For now, return a placeholder status
        tracing::debug!(
            message_id = %message_id,
            "Querying DIV UnifiedService status (not yet implemented)"
        );

        Ok(DeliveryStatus {
            transmission_id: message_id.to_string(),
            state: DeliveryState::InFlight,
            message: Some("Status query not yet implemented".to_string()),
        })
    }
}
