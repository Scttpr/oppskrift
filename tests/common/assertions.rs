//! Test assertion helpers for common validation patterns
//!
//! Provides reusable assertion macros and functions for testing API responses.

use serde_json::Value;

/// Assert that a JSON value contains a specific field
pub fn assert_json_field(value: &Value, field: &str) {
    assert!(
        value.get(field).is_some(),
        "Expected field '{}' to exist in JSON: {:?}",
        field,
        value
    );
}

/// Assert that a JSON value contains a field with a specific string value
pub fn assert_json_field_eq(value: &Value, field: &str, expected: &str) {
    let actual = value
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Expected field '{}' to exist as string in: {:?}",
                field, value
            )
        });
    assert_eq!(
        actual, expected,
        "Field '{}' expected '{}' but got '{}'",
        field, expected, actual
    );
}

/// Assert that a JSON value contains a field that is an array
pub fn assert_json_array(value: &Value, field: &str) {
    let arr = value
        .get(field)
        .unwrap_or_else(|| panic!("Expected field '{}' to exist in JSON: {:?}", field, value));
    assert!(
        arr.is_array(),
        "Expected field '{}' to be an array, got: {:?}",
        field,
        arr
    );
}

/// Assert that a JSON array field has a specific length
pub fn assert_json_array_len(value: &Value, field: &str, expected_len: usize) {
    let arr = value
        .get(field)
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("Expected field '{}' to be an array in: {:?}", field, value));
    assert_eq!(
        arr.len(),
        expected_len,
        "Expected array '{}' to have {} elements, got {}",
        field,
        expected_len,
        arr.len()
    );
}

/// Assert that a JSON value contains a non-empty string field
pub fn assert_json_non_empty_string(value: &Value, field: &str) {
    let s = value
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Expected field '{}' to exist as string in: {:?}",
                field, value
            )
        });
    assert!(!s.is_empty(), "Expected field '{}' to be non-empty", field);
}

/// Assert that a JSON value contains a valid UUID field
pub fn assert_json_uuid(value: &Value, field: &str) {
    let s = value
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Expected field '{}' to exist as string in: {:?}",
                field, value
            )
        });
    uuid::Uuid::parse_str(s)
        .unwrap_or_else(|_| panic!("Expected field '{}' to be a valid UUID, got: {}", field, s));
}

/// Assert that a JSON value contains a boolean field with expected value
pub fn assert_json_bool(value: &Value, field: &str, expected: bool) {
    let actual = value
        .get(field)
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| {
            panic!(
                "Expected field '{}' to exist as boolean in: {:?}",
                field, value
            )
        });
    assert_eq!(
        actual, expected,
        "Field '{}' expected {} but got {}",
        field, expected, actual
    );
}

/// Assert that a JSON value contains a numeric field
pub fn assert_json_number(value: &Value, field: &str) {
    let num = value
        .get(field)
        .unwrap_or_else(|| panic!("Expected field '{}' to exist in JSON: {:?}", field, value));
    assert!(
        num.is_number(),
        "Expected field '{}' to be a number, got: {:?}",
        field,
        num
    );
}

/// Assert that a JSON value contains an integer field with expected value
pub fn assert_json_i64(value: &Value, field: &str, expected: i64) {
    let actual = value
        .get(field)
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| {
            panic!(
                "Expected field '{}' to exist as integer in: {:?}",
                field, value
            )
        });
    assert_eq!(
        actual, expected,
        "Field '{}' expected {} but got {}",
        field, expected, actual
    );
}

/// Assert that an API response status indicates success (2xx)
pub fn assert_success_status(status: u16) {
    assert!(
        (200..300).contains(&status),
        "Expected success status (2xx), got {}",
        status
    );
}

/// Assert that an API response status indicates client error (4xx)
pub fn assert_client_error_status(status: u16) {
    assert!(
        (400..500).contains(&status),
        "Expected client error status (4xx), got {}",
        status
    );
}

/// Assert that an API response status indicates server error (5xx)
pub fn assert_server_error_status(status: u16) {
    assert!(
        (500..600).contains(&status),
        "Expected server error status (5xx), got {}",
        status
    );
}

/// Assert that a JSON error response contains an error message
pub fn assert_error_response(value: &Value) {
    assert!(
        value.get("message").is_some() || value.get("error").is_some(),
        "Expected error response to contain 'message' or 'error' field: {:?}",
        value
    );
}

/// Assert that a JSON response contains pagination metadata
pub fn assert_pagination_meta(value: &Value) {
    assert_json_field(value, "page");
    assert_json_field(value, "per_page");
    assert_json_field(value, "total");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_assert_json_field() {
        let value = json!({"name": "test", "count": 5});
        assert_json_field(&value, "name");
        assert_json_field(&value, "count");
    }

    #[test]
    #[should_panic(expected = "Expected field 'missing'")]
    fn test_assert_json_field_missing() {
        let value = json!({"name": "test"});
        assert_json_field(&value, "missing");
    }

    #[test]
    fn test_assert_json_array() {
        let value = json!({"items": [1, 2, 3]});
        assert_json_array(&value, "items");
    }

    #[test]
    fn test_assert_json_array_len() {
        let value = json!({"items": [1, 2, 3]});
        assert_json_array_len(&value, "items", 3);
    }

    #[test]
    fn test_assert_json_uuid() {
        let value = json!({"id": "550e8400-e29b-41d4-a716-446655440000"});
        assert_json_uuid(&value, "id");
    }

    #[test]
    fn test_assert_success_status() {
        assert_success_status(200);
        assert_success_status(201);
        assert_success_status(204);
    }

    #[test]
    fn test_assert_client_error_status() {
        assert_client_error_status(400);
        assert_client_error_status(401);
        assert_client_error_status(404);
    }

    #[test]
    fn test_assert_error_response() {
        let value = json!({"message": "Something went wrong"});
        assert_error_response(&value);

        let value2 = json!({"error": "Not found"});
        assert_error_response(&value2);
    }
}
