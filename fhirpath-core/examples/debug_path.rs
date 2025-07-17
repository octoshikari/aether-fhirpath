use fhirpath_core::evaluator::evaluate_expression;
use fhirpath_core::model::FhirPathValue;

fn main() {
    let resource = serde_json::json!({
        "resourceType": "Patient",
        "name": [
            {
                "given": ["John"],
                "family": "Doe"
            }
        ]
    });

    println!("Resource: {}", serde_json::to_string_pretty(&resource).unwrap());

    // Test simple property access
    println!("\n=== Testing 'resourceType' ===");
    match evaluate_expression("resourceType", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test array access
    println!("\n=== Testing 'name' ===");
    match evaluate_expression("name", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test path expression
    println!("\n=== Testing 'name.family' ===");
    match evaluate_expression("name.family", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test indexer
    println!("\n=== Testing 'name[0]' ===");
    match evaluate_expression("name[0]", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test indexer with path
    println!("\n=== Testing 'name[0].family' ===");
    match evaluate_expression("name[0].family", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }
}
