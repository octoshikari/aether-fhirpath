use fhirpath_core::evaluator::evaluate_expression;

fn main() {
    let resource = serde_json::json!({});

    println!("Testing XOR operations:");

    // Test true xor true (should be false)
    println!("\n=== Testing 'true xor true' ===");
    match evaluate_expression("true xor true", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test true xor false (should be true)
    println!("\n=== Testing 'true xor false' ===");
    match evaluate_expression("true xor false", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test false xor true (should be true)
    println!("\n=== Testing 'false xor true' ===");
    match evaluate_expression("false xor true", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test false xor false (should be false)
    println!("\n=== Testing 'false xor false' ===");
    match evaluate_expression("false xor false", resource.clone()) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test Rust's boolean XOR directly
    println!("\n=== Rust boolean XOR tests ===");
    println!("true ^ true = {}", true ^ true);
    println!("true ^ false = {}", true ^ false);
    println!("false ^ true = {}", false ^ true);
    println!("false ^ false = {}", false ^ false);
}
