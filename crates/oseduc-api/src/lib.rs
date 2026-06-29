#![forbid(unsafe_code)]

pub fn crate_name() -> &'static str {
    "oseduc-api"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "oseduc-api");
    }
}
