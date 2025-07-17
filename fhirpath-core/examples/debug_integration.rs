use fhirpath_core;
use std::fs;
use std::path::Path;

fn main() {
    println!("Debugging integration test failure...");

    // Load the patient fixture
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("patient-example.json");

    let fixture_content = fs::read_to_string(fixture_path).expect("Failed to read fixture file");
    let resource: serde_json::Value = serde_json::from_str(&fixture_content).expect("Failed to parse JSON");

    println!("Patient resource loaded successfully");

    // Test the path expression
    let expression = "Patient.name.given";
    println!("\nEvaluating expression: {}", expression);

    match fhirpath_core::evaluate(expression, resource.clone()) {
        Ok(result) => {
            println!("Result: {:?}", result);
            println!("Result type: {:?}", result);
            println!("Is array: {}", result.is_array());
            println!("Is null: {}", result.is_null());
            println!("Is string: {}", result.is_string());
            println!("Is object: {}", result.is_object());

            if result.is_array() {
                let array = result.as_array().unwrap();
                println!("Array length: {}", array.len());
                for (i, item) in array.iter().enumerate() {
                    println!("  [{}]: {:?}", i, item);
                }
            }
        },
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    // Also test simpler expressions to see what works
    println!("\n--- Testing simpler expressions ---");

    let simple_tests = [
        "Patient.name",
        "Patient.name[0]",
        "Patient.name[0].given",
        "Patient.resourceType",
    ];

    for expr in simple_tests.iter() {
        println!("\nTesting: {}", expr);
        match fhirpath_core::evaluate(expr, resource.clone()) {
            Ok(result) => {
                println!("  Result: {:?}", result);
                println!("  Type: {}", if result.is_array() { "array" } else if result.is_string() { "string" } else if result.is_object() { "object" } else { "other" });
            },
            Err(e) => {
                println!("  Error: {:?}", e);
            }
        }
    }
}
