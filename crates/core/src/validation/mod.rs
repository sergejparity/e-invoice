mod rules;
mod xsd;

pub fn validate(xml: &str) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    if let Err(e) = xsd::validate_against_xsd(xml) {
        errs.push(e);
    }
    if let Err(mut re) = rules::basic_en16931_checks(xml) {
        errs.append(&mut re);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}
