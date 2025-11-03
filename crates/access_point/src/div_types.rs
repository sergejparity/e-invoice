//! DIV UnifiedService XML types
//!
//! These types represent the DIV Envelope structure as defined in the XSD schemas.
//! This is a manual implementation based on the WSDL document.

use std::fmt;

/// DIV Envelope - the top-level structure for DIV messages
#[derive(Debug, Clone)]
pub struct DivEnvelope {
    /// Sender document section
    pub sender_document: SenderDocument,
}

/// Sender document section
#[derive(Debug, Clone)]
pub struct SenderDocument {
    /// Document metadata
    pub document_metadata: DocumentMetadata,
    /// Transport metadata
    pub sender_transport_metadata: SenderTransportMetadata,
}

/// Document metadata
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// General metadata
    pub general_metadata: GeneralMetadata,
    /// Payload reference
    pub payload_reference: Option<DocumentPayload>,
}

/// General metadata
#[derive(Debug, Clone)]
pub struct GeneralMetadata {
    /// Document authors
    pub authors: Authors,
    /// Document date (YYYY-MM-DD)
    pub date: String,
    /// Document kind
    pub document_kind: DocumentKind,
    /// Description
    pub description: Option<String>,
    /// Title
    pub title: String,
}

/// Authors collection
#[derive(Debug, Clone)]
pub struct Authors {
    /// Author entries
    pub author_entry: Vec<Correspondent>,
}

/// Document kind
#[derive(Debug, Clone)]
pub struct DocumentKind {
    /// Document kind code (e.g., "EINVOICE")
    pub document_kind_code: String,
    /// Document kind version
    pub document_kind_version: String,
    /// Optional name
    pub document_kind_name: Option<String>,
}

/// Document payload
#[derive(Debug, Clone)]
pub struct DocumentPayload {
    /// File entries
    pub file: Vec<FileEntry>,
}

/// File entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// MIME type
    pub mime_type: String,
    /// File size in bytes
    pub size: u64,
    /// File name
    pub name: String,
    /// Content reference
    pub content: ContentReference,
    /// Compression flag
    pub compressed: bool,
}

/// Content reference with digest
#[derive(Debug, Clone)]
pub struct ContentReference {
    /// Content ID
    pub content_reference: String,
    /// Digest value (base64)
    pub digest_value: String,
}

/// Sender transport metadata
#[derive(Debug, Clone)]
pub struct SenderTransportMetadata {
    /// Sender's e-adrese
    pub sender_e_address: String,
    /// Sender reference number
    pub sender_ref_number: String,
    /// Recipients
    pub recipients: Recipients,
    /// Notify on delivery
    pub notify_sender_on_delivery: bool,
    /// Priority level
    pub priority: String,
}

/// Recipients collection
#[derive(Debug, Clone)]
pub struct Recipients {
    /// Recipient entries
    pub recipient_entry: Vec<RecipientEntry>,
}

/// Individual recipient
#[derive(Debug, Clone)]
pub struct RecipientEntry {
    /// Recipient's e-adrese
    pub recipient_e_address: String,
}

/// Correspondent (institution or person)
#[derive(Debug, Clone)]
pub struct Correspondent {
    /// Institution
    pub institution: Option<InstitutionData>,
    /// Private person
    pub private_person: Option<PrivatePersonData>,
}

/// Institution data
#[derive(Debug, Clone)]
pub struct InstitutionData {
    /// Title/name
    pub title: String,
    /// Registration number
    pub registration_number: Option<String>,
}

/// Private person data
#[derive(Debug, Clone)]
pub struct PrivatePersonData {
    /// First name
    pub name: String,
    /// Surname
    pub surname: String,
}

impl DivEnvelope {
    /// Create a new DIV Envelope for an e-invoice
    pub fn new(
        title: String,
        date: String,
        sender_e_address: String,
        sender_ref_number: String,
        recipient_e_address: String,
        sender_org_name: String,
        file_name: String,
        mime_type: String,
        file_size: u64,
        digest_value: String,
    ) -> Self {
        DivEnvelope {
            sender_document: SenderDocument {
                document_metadata: DocumentMetadata {
                    general_metadata: GeneralMetadata {
                        authors: Authors {
                            author_entry: vec![Correspondent {
                                institution: Some(InstitutionData {
                                    title: sender_org_name,
                                    registration_number: None,
                                }),
                                private_person: None,
                            }],
                        },
                        date,
                        document_kind: DocumentKind {
                            document_kind_code: "EINVOICE".to_string(),
                            document_kind_version: "1.0".to_string(),
                            document_kind_name: Some("E-invoice".to_string()),
                        },
                        description: None,
                        title,
                    },
                    payload_reference: Some(DocumentPayload {
                        file: vec![FileEntry {
                            mime_type: mime_type.clone(),
                            size: file_size,
                            name: file_name,
                            content: ContentReference {
                                content_reference: "cid:invoice-content".to_string(),
                                digest_value,
                            },
                            compressed: false,
                        }],
                    }),
                },
                sender_transport_metadata: SenderTransportMetadata {
                    sender_e_address,
                    sender_ref_number,
                    recipients: Recipients {
                        recipient_entry: vec![RecipientEntry {
                            recipient_e_address,
                        }],
                    },
                    notify_sender_on_delivery: true,
                    priority: "normal".to_string(),
                },
            },
        }
    }

    /// Serialize to XML string
    pub fn to_xml(&self) -> String {
        format!(
            r#"<Envelope xmlns="http://ivis.eps.gov.lv/XMLSchemas/100001/DIV/v1-0">
  <SenderDocument Id="SenderSection">
    <DocumentMetadata>
      <GeneralMetadata>
        <Title>{}</Title>
        <Date>{}</Date>
        <DocumentKind>
          <DocumentKindCode>EINVOICE</DocumentKindCode>
          <DocumentKindVersion>1.0</DocumentKindVersion>
          <DocumentKindName>E-invoice</DocumentKindName>
        </DocumentKind>
        <Authors>
          <AuthorEntry>
            <Institution>
              <Title>{}</Title>
            </Institution>
          </AuthorEntry>
        </Authors>
      </GeneralMetadata>
      <PayloadReference>
        <File>
          <MimeType>{}</MimeType>
          <Size>{}</Size>
          <Name>{}</Name>
          <Content>
            <ContentReference>cid:invoice-content</ContentReference>
            <DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
            <DigestValue>{}</DigestValue>
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
      <Priority>{}</Priority>
    </SenderTransportMetadata>
  </SenderDocument>
</Envelope>"#,
            self.sender_document.document_metadata.general_metadata.title,
            self.sender_document.document_metadata.general_metadata.date,
            self.sender_document.document_metadata.general_metadata.authors.author_entry[0].institution.as_ref().unwrap().title,
            self.sender_document.document_metadata.payload_reference.as_ref().unwrap().file[0].mime_type,
            self.sender_document.document_metadata.payload_reference.as_ref().unwrap().file[0].size,
            self.sender_document.document_metadata.payload_reference.as_ref().unwrap().file[0].name,
            self.sender_document.document_metadata.payload_reference.as_ref().unwrap().file[0].content.digest_value,
            self.sender_document.sender_transport_metadata.sender_e_address,
            self.sender_document.sender_transport_metadata.sender_ref_number,
            self.sender_document.sender_transport_metadata.recipients.recipient_entry[0].recipient_e_address,
            self.sender_document.sender_transport_metadata.priority,
        )
    }
}

impl fmt::Display for DivEnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_xml())
    }
}

/// Compute SHA-256 digest in base64 format
pub fn compute_sha256_base64(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    base64::encode(&hash)
}