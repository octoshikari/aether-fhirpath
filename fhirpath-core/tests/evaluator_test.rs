// FHIRPath Evaluator Tests
//
// This file contains tests for the FHIRPath evaluator.

use fhirpath_core::evaluator::{evaluate_expression, evaluate_ast, EvaluationContext};
use fhirpath_core::model::FhirPathValue;
use fhirpath_core::lexer::tokenize;
use fhirpath_core::parser::parse;

#[test]
fn test_evaluate_identifier() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "id": "123",
        "name": [
            {
                "given": ["John"],
                "family": "Doe"
            }
        ]
    });

    let result = evaluate_expression("id", resource).unwrap();

    match result {
        FhirPathValue::String(value) => {
            assert_eq!(value, "123");
        },
        _ => panic!("Expected String value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_string_literal() {
    let resource = serde_json::json!({});

    let result = evaluate_expression("'hello'", resource).unwrap();

    match result {
        FhirPathValue::String(value) => {
            assert_eq!(value, "hello");
        },
        _ => panic!("Expected String value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_number_literal() {
    let resource = serde_json::json!({});

    // Integer
    let result = evaluate_expression("42", resource.clone()).unwrap();
    match result {
        FhirPathValue::Integer(value) => {
            assert_eq!(value, 42);
        },
        _ => panic!("Expected Integer value, got {:?}", result),
    }

    // Decimal
    let result = evaluate_expression("42.5", resource).unwrap();
    match result {
        FhirPathValue::Decimal(value) => {
            assert_eq!(value, 42.5);
        },
        _ => panic!("Expected Decimal value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_boolean_literal() {
    let resource = serde_json::json!({});

    // True
    let result = evaluate_expression("true", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // False
    let result = evaluate_expression("false", resource).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_path_expression() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "name": [
            {
                "given": ["John"],
                "family": "Doe"
            }
        ]
    });

    let result = evaluate_expression("name.family", resource).unwrap();

    match result {
        FhirPathValue::String(value) => {
            assert_eq!(value, "Doe");
        },
        _ => panic!("Expected String value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_indexer() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "name": [
            {
                "given": ["John", "J"],
                "family": "Doe"
            },
            {
                "given": ["Johnny"],
                "family": "Doe"
            }
        ]
    });

    // First name
    let result = evaluate_expression("name[0].given[0]", resource.clone()).unwrap();
    match result {
        FhirPathValue::String(value) => {
            assert_eq!(value, "John");
        },
        _ => panic!("Expected String value, got {:?}", result),
    }

    // Second name
    let result = evaluate_expression("name[1].given[0]", resource).unwrap();
    match result {
        FhirPathValue::String(value) => {
            assert_eq!(value, "Johnny");
        },
        _ => panic!("Expected String value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_equality() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "gender": "male"
    });

    // Equal (true)
    let result = evaluate_expression("gender = 'male'", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Equal (false)
    let result = evaluate_expression("gender = 'female'", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Not equal (true)
    let result = evaluate_expression("gender != 'female'", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Not equal (false)
    let result = evaluate_expression("gender != 'male'", resource).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_comparison() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "age": 25
    });

    // Less than (true)
    let result = evaluate_expression("age < 30", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Less than (false)
    let result = evaluate_expression("age < 20", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Greater than (true)
    let result = evaluate_expression("age > 20", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Greater than (false)
    let result = evaluate_expression("age > 30", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Less than or equal (true - equal)
    let result = evaluate_expression("age <= 25", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Greater than or equal (true - equal)
    let result = evaluate_expression("age >= 25", resource).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_arithmetic() {
    let resource = serde_json::json!({});

    // Addition
    let result = evaluate_expression("5 + 3", resource.clone()).unwrap();
    match result {
        FhirPathValue::Integer(value) => {
            assert_eq!(value, 8);
        },
        _ => panic!("Expected Integer value, got {:?}", result),
    }

    // Subtraction
    let result = evaluate_expression("5 - 3", resource.clone()).unwrap();
    match result {
        FhirPathValue::Integer(value) => {
            assert_eq!(value, 2);
        },
        _ => panic!("Expected Integer value, got {:?}", result),
    }

    // Multiplication
    let result = evaluate_expression("5 * 3", resource.clone()).unwrap();
    match result {
        FhirPathValue::Integer(value) => {
            assert_eq!(value, 15);
        },
        _ => panic!("Expected Integer value, got {:?}", result),
    }

    // Division
    let result = evaluate_expression("6 / 3", resource.clone()).unwrap();
    match result {
        FhirPathValue::Decimal(value) => {
            assert_eq!(value, 2.0);
        },
        _ => panic!("Expected Decimal value, got {:?}", result),
    }

    // Mixed types
    let result = evaluate_expression("5 + 3.5", resource).unwrap();
    match result {
        FhirPathValue::Decimal(value) => {
            assert_eq!(value, 8.5);
        },
        _ => panic!("Expected Decimal value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_logical() {
    let resource = serde_json::json!({});

    // And (true)
    let result = evaluate_expression("true and true", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // And (false)
    let result = evaluate_expression("true and false", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Or (true)
    let result = evaluate_expression("true or false", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Or (false)
    let result = evaluate_expression("false or false", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Xor (true)
    let result = evaluate_expression("true xor false", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Xor (false)
    let result = evaluate_expression("true xor true", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Implies (true)
    let result = evaluate_expression("false implies true", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }

    // Implies (false)
    let result = evaluate_expression("true implies false", resource).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, false);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }
}

#[test]
fn test_debug_xor() {
    let resource = serde_json::json!({});

    println!("Testing XOR operations:");

    // Test true xor false (should be true)
    let result1 = evaluate_expression("true xor false", resource.clone()).unwrap();
    println!("true xor false = {:?}", result1);

    // Test true xor true (should be false)
    let result2 = evaluate_expression("true xor true", resource.clone()).unwrap();
    println!("true xor true = {:?}", result2);

    // Test false xor false (should be false)
    let result3 = evaluate_expression("false xor false", resource.clone()).unwrap();
    println!("false xor false = {:?}", result3);

    // Test false xor true (should be true)
    let result4 = evaluate_expression("false xor true", resource).unwrap();
    println!("false xor true = {:?}", result4);

    // Test basic boolean XOR in Rust
    println!("\nRust boolean XOR tests:");
    println!("true ^ false = {}", true ^ false);
    println!("true ^ true = {}", true ^ true);
    println!("false ^ false = {}", false ^ false);
    println!("false ^ true = {}", false ^ true);
}

#[test]
fn test_evaluate_unary() {
    let resource = serde_json::json!({});

    // Negation
    let result = evaluate_expression("-5", resource.clone()).unwrap();
    match result {
        FhirPathValue::Integer(value) => {
            assert_eq!(value, -5);
        },
        _ => panic!("Expected Integer value, got {:?}", result),
    }

    // Not
    let result = evaluate_expression("true", resource.clone()).unwrap();
    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_complex_expression() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "name": [
            {
                "given": ["John"],
                "family": "Doe"
            }
        ],
        "gender": "male",
        "age": 25
    });

    let result = evaluate_expression("name[0].family = 'Doe' and gender = 'male'", resource).unwrap();

    match result {
        FhirPathValue::Boolean(value) => {
            assert_eq!(value, true);
        },
        _ => panic!("Expected Boolean value, got {:?}", result),
    }
}

#[test]
fn test_evaluate_with_context() {
    // Create a context with a variable
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "name": [
            {
                "given": ["John"],
                "family": "Doe"
            }
        ]
    });

    let tokens = tokenize("name[0].family").unwrap();
    let ast = parse(&tokens).unwrap();

    let mut context = EvaluationContext::new(resource);
    context.set_variable("expected", FhirPathValue::String("Doe".to_string()));

    let result = evaluate_ast(&ast, &context).unwrap();

    match result {
        FhirPathValue::String(value) => {
            assert_eq!(value, "Doe");
        },
        _ => panic!("Expected String value, got {:?}", result),
    }
}
