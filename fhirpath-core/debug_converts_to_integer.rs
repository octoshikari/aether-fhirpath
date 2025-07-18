use fhirpath_core::evaluator::evaluate_expression;
use fhirpath_core::model::FhirPathValue;

fn main() {
    // Test the convertsToInteger function
    let test_cases = vec![
        "1.convertsToInteger()",
        "0.convertsToInteger()",
        "(-1).convertsToInteger()",
        "1.0.convertsToInteger()",
        "1.5.convertsToInteger()",
        "'1'.convertsToInteger()",
        "'abc'.convertsToInteger()",
        "true.convertsToInteger()",
    ];

    for expression in test_cases {
        match evaluate_expression(expression, serde_json::json!({})) {
            Ok(result) => {
                println!("Expression: {} -> Result: {:?}", expression, result);
            }
            Err(e) => {
                println!("Expression: {} -> Error: {:?}", expression, e);
            }
        }
    }

    // Also test what type 1 evaluates to
    match evaluate_expression("1", serde_json::json!({})) {
        Ok(result) => {
            println!("Expression: 1 -> Result: {:?}", result);
        }
        Err(e) => {
            println!("Expression: 1 -> Error: {:?}", e);
        }
    }
}
