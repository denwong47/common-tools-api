//! Traits for working with HTTP request query models.

use serde::Serialize;

use crate::error::{Error, Result};

/// Converts a JSON scalar value into a query-compatible format.
///
/// This will error on non-scalar JSON values such as objects and arrays.
fn json_scalar_to_query_value(value: serde_json::Value) -> Result<Option<String>> {
    match value {
        serde_json::Value::Null => Ok(None),
        // This is the same as `to_string`, but allows some more control over the output format in the future.
        serde_json::Value::Bool(b) => Ok(if b {
            Some("true".to_string())
        } else {
            Some("false".to_string())
        }),
        // Numbers are converted to their string representation.
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        serde_json::Value::String(s) => Ok(Some(s)),
        _ => Err(Error::InvalidQueryValue {
            action: "converting scalar to query value",
            message: format!("expected a scalar value, got {:?}", value),
        }),
    }
}

/// A trait for types that can be exported as HTTP query parameters.
#[allow(unused)]
pub trait CanExportQuery: Serialize {
    /// Converts the implementing type into a Query compatible format.
    fn to_query(&self) -> Result<Vec<(String, String)>> {
        let json = serde_json::to_value(self).map_err(|e| Error::InvalidQueryValue {
            action: "serializing into object",
            message: e.to_string(),
        })?;

        if let serde_json::Value::Object(map) = json {
            let mut query = Vec::new();
            for (k, v) in map {
                match v {
                    // Exception at root level - arrays are expanded into multiple query parameters with the same key.
                    serde_json::Value::Array(items) => {
                        for item in items {
                            if let Some(value) = json_scalar_to_query_value(item)? {
                                query.push((k.clone(), value))
                            }
                        }
                    }
                    _ => {
                        if let Some(value) = json_scalar_to_query_value(v)? {
                            query.push((k.clone(), value))
                        }
                    }
                }
            }
            Ok(query)
        } else {
            Err(Error::InvalidQueryValue {
                action: "converting to query",
                message: format!("expected an object with fields and values, got {:?}", json),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde::Serialize;

    mod good_struct_tests {
        use super::*;

        #[derive(Serialize)]
        struct GoodStruct {
            str: String,
            num: i32,
            arr: Vec<String>,
            optional: Option<String>,
        }

        impl CanExportQuery for GoodStruct {}

        macro_rules! create_test {
            (
                $name:ident({
                    str: $str:literal,
                    num: $num:literal,
                    arr: $arr:expr,
                    optional: $optional:expr
                }) -> $expected:expr
            ) => {
                #[test]
                fn $name() {
                    let instance = GoodStruct {
                        str: $str.to_string(),
                        num: $num,
                        arr: $arr,
                        optional: $optional,
                    };
                    let query = instance.to_query().unwrap();
                    assert_eq!(query, $expected);
                }
            };
        }

        create_test!(
            test_all_present({
                str: "test",
                num: 42,
                arr: vec!["item1".to_string(), "item2".to_string()],
                optional: Some("optional_value".to_string())
            }) -> vec![
                ("str".to_string(), "test".to_string()),
                ("num".to_string(), "42".to_string()),
                ("arr".to_string(), "item1".to_string()),
                ("arr".to_string(), "item2".to_string()),
                ("optional".to_string(), "optional_value".to_string())
            ]
        );

        create_test!(
            test_optional_none({
                str: "test",
                num: 42,
                arr: vec!["item1".to_string(), "item2".to_string()],
                optional: None
            }) -> vec![
                ("str".to_string(), "test".to_string()),
                ("num".to_string(), "42".to_string()),
                ("arr".to_string(), "item1".to_string()),
                ("arr".to_string(), "item2".to_string())
            ]
        );

        create_test!(
            test_empty_array({
                str: "test",
                num: 42,
                arr: vec![],
                optional: None
            }) -> vec![
                ("str".to_string(), "test".to_string()),
                ("num".to_string(), "42".to_string())
            ]
        );
    }

    mod bad_struct_tests {
        use super::*;

        use crate::Error;

        #[derive(Serialize)]
        struct NestedJsonStruct {
            str: String,
            value: serde_json::Value,
        }

        impl CanExportQuery for NestedJsonStruct {}

        #[derive(Serialize)]
        #[serde(transparent)] // This will cause serialization to treat the struct as a single value, which is not compatible with query export
        struct BadStruct(Vec<u8>);

        impl CanExportQuery for BadStruct {}

        macro_rules! create_test {
            (
                $name:ident({
                    value: $value:expr
                }) -> {
                    action: $action:literal,
                    message: $message:literal
                }
            ) => {
                #[test]
                fn $name() {
                    let instance = NestedJsonStruct {
                        str: "test".to_string(),
                        value: $value,
                    };
                    let result = instance.to_query();
                    assert!(result.is_err());
                    if let Err(Error::InvalidQueryValue { action, message }) = result {
                        assert_eq!(action, $action);
                        assert_eq!(message, $message);
                    } else {
                        panic!("Expected Err(Error::InvalidQueryValue {{ action, message }}), got {:?}", result);
                    }
                }
            };
        }

        create_test!(
            test_nested_object({
                value: serde_json::json!({"key": "value"})
            }) -> {
                action: "converting scalar to query value",
                message: "expected a scalar value, got Object {\"key\": String(\"value\")}"
            }
        );

        #[test]
        fn test_array_struct() {
            let instance = BadStruct(vec![1, 2, 3]);
            let result = instance.to_query();
            assert!(result.is_err());
            if let Err(Error::InvalidQueryValue { action, message }) = result {
                assert_eq!(action, "converting to query");
                assert_eq!(
                    message,
                    "expected an object with fields and values, got Array [Number(1), Number(2), Number(3)]"
                );
            } else {
                panic!(
                    "Expected Err(Error::InvalidQueryValue {{ action, message }}), got {:?}",
                    result
                );
            }
        }
    }
}
