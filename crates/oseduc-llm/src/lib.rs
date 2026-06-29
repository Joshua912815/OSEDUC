#![forbid(unsafe_code)]

pub fn crate_name() -> &'static str {
    "oseduc-llm"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "oseduc-llm");
    }
}
