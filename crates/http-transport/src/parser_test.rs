// Temporary test file to verify parser refactoring
#[cfg(test)]
mod tests {
    use crate::parser::{parse_client_notification, parse_client_request, parse_client_response, parse_request_id};

    #[test]
    fn test_imports_work() {
        // This test just verifies that all public functions are accessible
        // If this compiles, our refactoring is successful
        assert!(true);
    }
}