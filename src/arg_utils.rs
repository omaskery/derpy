
pub fn parse_option_key_value(text: &str) -> Result<(String, String), String> {
    let parts = text.splitn(2, ":")
        .collect::<Vec<_>>();

    if parts.len() == 2 {
        Ok((parts[0].into(), parts[1].into()))
    } else {
        Err("key value pair must be two strings separated by a ':' character".into())
    }
}

pub fn validate_option_key_value(text: String) -> Result<(), String> {
    match parse_option_key_value(&text) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

