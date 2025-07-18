use fhirpath_core::evaluator::evaluate_expression;
use serde_json::Value;

fn main() {
    // Test a simple DateTime literal
    let patient_json = r#"{"resourceType": "Patient", "id": "example"}"#;
    let patient_data: Value = serde_json::from_str(patient_json).unwrap();

    // Test cases from the official tests
    let test_cases = vec![
        "@2015T.is(DateTime)",
        "@2015-02T.is(DateTime)",
        "@2015-02-04T14:34:28.is(DateTime)",
        "@2015-02-04T14:34:28Z.is(DateTime)",
        "'2015'.convertsToDateTime()",
    ];

    for expression in test_cases {
        println!("Testing expression: {}", expression);
        match evaluate_expression(expression, patient_data.clone()) {
            Ok(result) => {
                println!("  Result: {:?}", result);
            }
            Err(e) => {
                println!("  Error: {:?}", e);
            }
        }
        println!();
    }
}
