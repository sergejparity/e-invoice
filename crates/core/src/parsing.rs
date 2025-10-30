use anyhow::Result;
use sha2::{Digest, Sha256};

pub fn compute_sha256_hex(xml: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(xml.as_bytes());
    let bytes = hasher.finalize();
    hex::encode(bytes)
}

fn find_element<'a, 'input: 'a>(
    node: roxmltree::Node<'a, 'input>,
    path: &[&str],
) -> Option<roxmltree::Node<'a, 'input>> {
    if path.is_empty() {
        return Some(node);
    }
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == path[0] {
            if path.len() == 1 {
                return Some(child);
            }
            if let Some(found) = find_element(child, &path[1..]) {
                return Some(found);
            }
        }
    }
    None
}

fn get_text_at_path(doc: &roxmltree::Document, path: &[&str]) -> Option<String> {
    find_element(doc.root_element(), path)
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Debug, Clone)]
pub struct UBLInvoice {
    pub invoice_number: String,
    pub issue_date: String,
    pub due_date: Option<String>,
    pub currency_code: String,
    pub supplier_name: String,
    pub supplier_id: Option<String>,
    pub customer_name: String,
    pub customer_id: Option<String>,
    pub tax_total: Option<String>,
    pub payable_amount: Option<String>,
}

pub fn parse_ubl_invoice(xml: &str) -> Result<UBLInvoice> {
    let doc = roxmltree::Document::parse(xml)?;

    let invoice_number = get_text_at_path(&doc, &["ID"]).unwrap_or_else(|| "UNKNOWN".to_string());
    let issue_date = get_text_at_path(&doc, &["IssueDate"]).unwrap_or_default();
    let due_date = get_text_at_path(&doc, &["DueDate"]);
    let currency_code = get_text_at_path(&doc, &["DocumentCurrencyCode"]).unwrap_or_default();

    let supplier_name = get_text_at_path(
        &doc,
        &["AccountingSupplierParty", "Party", "PartyName", "Name"],
    )
    .or_else(|| {
        get_text_at_path(
            &doc,
            &[
                "AccountingSupplierParty",
                "Party",
                "PartyLegalEntity",
                "RegistrationName",
            ],
        )
    })
    .unwrap_or_default();
    let supplier_id = get_text_at_path(&doc, &["AccountingSupplierParty", "Party", "EndpointID"]);

    let customer_name = get_text_at_path(
        &doc,
        &["AccountingCustomerParty", "Party", "PartyName", "Name"],
    )
    .or_else(|| {
        get_text_at_path(
            &doc,
            &[
                "AccountingCustomerParty",
                "Party",
                "PartyLegalEntity",
                "RegistrationName",
            ],
        )
    })
    .unwrap_or_default();
    let customer_id = get_text_at_path(&doc, &["AccountingCustomerParty", "Party", "EndpointID"]);

    let tax_total = get_text_at_path(&doc, &["TaxTotal", "TaxAmount"]);
    let payable_amount = get_text_at_path(&doc, &["LegalMonetaryTotal", "PayableAmount"]);

    Ok(UBLInvoice {
        invoice_number,
        issue_date,
        due_date,
        currency_code,
        supplier_name,
        supplier_id,
        customer_name,
        customer_id,
        tax_total,
        payable_amount,
    })
}
