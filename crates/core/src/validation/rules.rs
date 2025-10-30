use crate::parsing::parse_ubl_invoice;

pub fn basic_en16931_checks(xml: &str) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();

    // Check root element
    if !xml.contains("<Invoice") && !xml.contains("<invoice") {
        errs.push("Missing UBL Invoice root element".to_string());
        return Err(errs);
    }

    // Parse and validate mandatory fields per EN16931
    let invoice = match parse_ubl_invoice(xml) {
        Ok(inv) => inv,
        Err(e) => {
            errs.push(format!("Failed to parse UBL: {}", e));
            return Err(errs);
        }
    };

    // BT-1: Invoice number (mandatory)
    if invoice.invoice_number.is_empty() || invoice.invoice_number == "UNKNOWN" {
        errs.push("BT-1: Invoice number is mandatory".to_string());
    }

    // BT-2: Issue date (mandatory)
    if invoice.issue_date.is_empty() {
        errs.push("BT-2: Issue date is mandatory".to_string());
    }

    // BT-5: Invoice currency code (mandatory)
    if invoice.currency_code.is_empty() {
        errs.push("BT-5: Currency code is mandatory".to_string());
    } else if invoice.currency_code.len() != 3 {
        errs.push("BT-5: Currency code must be 3 characters (ISO 4217)".to_string());
    }

    // BG-4: Seller (mandatory)
    if invoice.supplier_name.is_empty() {
        errs.push("BG-4: Seller name is mandatory".to_string());
    }

    // BG-7: Buyer (mandatory)
    if invoice.customer_name.is_empty() {
        errs.push("BG-7: Buyer name is mandatory".to_string());
    }

    // BT-115: Payable amount should be present
    if invoice.payable_amount.is_none() {
        errs.push("BT-115: Payable amount should be present".to_string());
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}
