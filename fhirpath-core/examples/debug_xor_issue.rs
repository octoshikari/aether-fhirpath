use fhirpath_core::evaluator::evaluate_expression;
use fhirpath_core::model::FhirPathValue;
use serde_json::json;

fn main() {
    let resource = json!({});

    println!("Testing XOR operations step by step:");

    // Test individual boolean literals first
    println!("\n1. Testing boolean literals:");
    let true_result = evaluate_expression("true", resource.clone()).unwrap();
    let false_result = evaluate_expression("false", resource.clone()).unwrap();
    println!("true = {:?}", true_result);
    println!("false = {:?}", false_result);

    // Test XOR operations
    println!("\n2. Testing XOR operations:");

    let test_cases = [
        ("true xor false", true),
        ("false xor true", true),
        ("true xor true", false),
        ("false xor false", false),
    ];

    for (expr, expected) in test_cases.iter() {
        match evaluate_expression(expr, resource.clone()) {
            Ok(FhirPathValue::Boolean(result)) => {
                println!("{} = {} (expected: {})", expr, result, expected);
                if result != *expected {
                    println!("  ❌ MISMATCH!");
                } else {
                    println!("  ✅ CORRECT");
                }
            },
            Ok(other) => {
                println!("{} = {:?} (expected Boolean)", expr, other);
                println!("  ❌ WRONG TYPE!");
            },
            Err(e) => {
                println!("{} = ERROR: {:?}", expr, e);
                println!("  ❌ ERROR!");
            }
        }
    }

    // Test Rust XOR for comparison
    println!("\n3. Rust XOR for comparison:");
    println!("true ^ false = {}", true ^ false);
    println!("false ^ true = {}", false ^ true);
    println!("true ^ true = {}", true ^ true);
    println!("false ^ false = {}", false ^ false);
}
