use fhirpath_core::evaluator::evaluate_expression;
use serde_json::json;

#[test]
fn debug_complex_expression() {
    let patient = json!({
        "resourceType": "Patient",
        "id": "example",
        "name": [
            {
                "use": "official",
                "family": "Smith",
                "given": ["John", "Adam"]
            }
        ],
        "gender": "male",
        "birthDate": "1974-12-25"
    });

    // Test each part of the complex expression separately
    println!("Testing individual parts:");

    let expr1 = "Patient.name.where(given[0] = 'John').exists()";
    match evaluate_expression(expr1, patient.clone()) {
        Ok(result) => println!("Part 1 ({}): {:?}", expr1, result),
        Err(e) => println!("Part 1 ERROR: {}", e),
    }

    let expr2 = "Patient.gender = 'male'";
    match evaluate_expression(expr2, patient.clone()) {
        Ok(result) => println!("Part 2 ({}): {:?}", expr2, result),
        Err(e) => println!("Part 2 ERROR: {}", e),
    }

    // Test Patient.birthDate first
    let expr3a = "Patient.birthDate";
    match evaluate_expression(expr3a, patient.clone()) {
        Ok(result) => println!("Part 3a ({}): {:?}", expr3a, result),
        Err(e) => println!("Part 3a ERROR: {}", e),
    }

    let expr3 = "Patient.birthDate.exists()";
    match evaluate_expression(expr3, patient.clone()) {
        Ok(result) => println!("Part 3 ({}): {:?}", expr3, result),
        Err(e) => println!("Part 3 ERROR: {}", e),
    }

    // Test combinations
    println!("\nTesting combinations:");

    let expr_combo1 = "Patient.name.where(given[0] = 'John').exists() and Patient.gender = 'male'";
    match evaluate_expression(expr_combo1, patient.clone()) {
        Ok(result) => println!("Combo 1: {:?}", result),
        Err(e) => println!("Combo 1 ERROR: {}", e),
    }

    let expr_full = "Patient.name.where(given[0] = 'John').exists() and Patient.gender = 'male' and Patient.birthDate.exists()";
    match evaluate_expression(expr_full, patient.clone()) {
        Ok(result) => println!("Full expression: {:?}", result),
        Err(e) => println!("Full expression ERROR: {}", e),
    }
}
