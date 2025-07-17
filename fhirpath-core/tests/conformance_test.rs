// FHIRPath Conformance Tests
//
// This file contains conformance tests against the FHIRPath specification examples.
// These tests ensure our implementation correctly handles the standard FHIRPath expressions
// as defined in the official specification.

use fhirpath_core::evaluator::evaluate_expression;
use fhirpath_core::model::FhirPathValue;
use serde_json::Value;
use std::fs;

fn load_patient_fixture() -> Value {
    let fixture_path = "tests/fixtures/patient-example.json";
    let fixture_content = fs::read_to_string(fixture_path)
        .expect("Failed to read patient fixture file");
    serde_json::from_str(&fixture_content)
        .expect("Failed to parse patient fixture JSON")
}

#[test]
fn test_conformance_basic_property_access() {
    let patient = load_patient_fixture();

    // Test resourceType access
    let result = evaluate_expression("resourceType", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "Patient"),
                _ => panic!("Expected string value for resourceType"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test id access
    let result = evaluate_expression("id", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "example"),
                _ => panic!("Expected string value for id"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test gender access
    let result = evaluate_expression("gender", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "male"),
                _ => panic!("Expected string value for gender"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test active status
    let result = evaluate_expression("active", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::Boolean(b) => assert_eq!(*b, true),
                _ => panic!("Expected boolean value for active"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_array_access() {
    let patient = load_patient_fixture();

    // Test name array access
    let result = evaluate_expression("name", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 2); // Should have 2 name entries
        }
        _ => panic!("Expected collection result for name"),
    }

    // Test identifier array access
    let result = evaluate_expression("identifier", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1); // Should have 1 identifier entry
        }
        _ => panic!("Expected collection result for identifier"),
    }

    // Test telecom array access
    let result = evaluate_expression("telecom", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 2); // Should have 2 telecom entries
        }
        _ => panic!("Expected collection result for telecom"),
    }
}

#[test]
fn test_conformance_path_expressions() {
    let patient = load_patient_fixture();

    // Test name.family path
    let result = evaluate_expression("name.family", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "Smith"),
                _ => panic!("Expected string value for name.family"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test identifier.value path
    let result = evaluate_expression("identifier.value", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "12345"),
                _ => panic!("Expected string value for identifier.value"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test telecom.system path
    let result = evaluate_expression("telecom.system", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 2);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "phone"),
                _ => panic!("Expected string value for first telecom.system"),
            }
            match &values[1] {
                FhirPathValue::String(s) => assert_eq!(s, "email"),
                _ => panic!("Expected string value for second telecom.system"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_indexer_expressions() {
    let patient = load_patient_fixture();

    // Test first name entry
    let result = evaluate_expression("name[0]", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            // The result should be a resource representing the first name entry
        }
        _ => panic!("Expected collection result for name[0]"),
    }

    // Test first name's family
    let result = evaluate_expression("name[0].family", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "Smith"),
                _ => panic!("Expected string value for name[0].family"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test first telecom entry system
    let result = evaluate_expression("telecom[0].system", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "phone"),
                _ => panic!("Expected string value for telecom[0].system"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_nested_path_expressions() {
    let patient = load_patient_fixture();

    // Test nested identifier type coding system
    let result = evaluate_expression("identifier.type.coding.system", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert!(values.len() >= 1);
            // Should contain the coding system value
        }
        _ => panic!("Expected collection result"),
    }

    // Test contact name family
    let result = evaluate_expression("contact.name.family", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "Smith"),
                _ => panic!("Expected string value for contact.name.family"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_meta_properties() {
    let patient = load_patient_fixture();

    // Test meta.versionId
    let result = evaluate_expression("meta.versionId", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "1"),
                _ => panic!("Expected string value for meta.versionId"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test text.status
    let result = evaluate_expression("text.status", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "generated"),
                _ => panic!("Expected string value for text.status"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_address_expressions() {
    let patient = load_patient_fixture();

    // Test address array
    let result = evaluate_expression("address", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1); // Should have 1 address entry
        }
        _ => panic!("Expected collection result for address"),
    }

    // Test address.city
    let result = evaluate_expression("address.city", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "Anytown"),
                _ => panic!("Expected string value for address.city"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test address.state
    let result = evaluate_expression("address.state", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "CA"),
                _ => panic!("Expected string value for address.state"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_empty_results() {
    let patient = load_patient_fixture();

    // Test non-existent property
    let result = evaluate_expression("nonExistentProperty", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 0); // Should be empty
        }
        _ => panic!("Expected collection result"),
    }

    // Test out-of-bounds indexer
    let result = evaluate_expression("name[10]", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 0); // Should be empty
        }
        _ => panic!("Expected collection result"),
    }
}

#[test]
fn test_conformance_complex_expressions() {
    let patient = load_patient_fixture();

    // Test multiple levels of nesting with indexing
    let result = evaluate_expression("name[0].given[0]", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "John"),
                _ => panic!("Expected string value for name[0].given[0]"),
            }
        }
        _ => panic!("Expected collection result"),
    }

    // Test identifier system access
    let result = evaluate_expression("identifier.system", patient.clone()).unwrap();
    match result {
        FhirPathValue::Collection(values) => {
            assert_eq!(values.len(), 1);
            match &values[0] {
                FhirPathValue::String(s) => assert_eq!(s, "urn:oid:1.2.36.146.595.217.0.1"),
                _ => panic!("Expected string value for identifier.system"),
            }
        }
        _ => panic!("Expected collection result"),
    }
}
