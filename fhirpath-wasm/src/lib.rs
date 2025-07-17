use wasm_bindgen::prelude::*;
use fhirpath_core;
use serde_json;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// This is like the `extern` block in C.
#[wasm_bindgen]
extern "C" {
    // Bind the `console.log` function from the browser's console
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Define a macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! console_log {
    ( $( $t:tt )* ) => {
        log(&format!( $( $t )* ))
    }
}

/// Initialize panic hook for better error messages in the browser
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Evaluate a FHIRPath expression against a FHIR resource
///
/// # Arguments
/// * `expression` - The FHIRPath expression to evaluate
/// * `resource_json` - The FHIR resource as a JSON string
///
/// # Returns
/// A JSON string containing the evaluation result, or an error message
#[wasm_bindgen]
pub fn evaluate_fhirpath(expression: &str, resource_json: &str) -> String {
    // Parse the JSON resource
    let resource: serde_json::Value = match serde_json::from_str(resource_json) {
        Ok(value) => value,
        Err(e) => {
            return format!(r#"{{"error": "Invalid JSON resource: {}"}}"#, e);
        }
    };

    // Evaluate the FHIRPath expression
    match fhirpath_core::evaluate(expression, resource) {
        Ok(result) => {
            match serde_json::to_string(&result) {
                Ok(json_str) => json_str,
                Err(e) => format!(r#"{{"error": "Failed to serialize result: {}"}}"#, e),
            }
        }
        Err(e) => {
            format!(r#"{{"error": "FHIRPath evaluation error: {}"}}"#, e)
        }
    }
}

/// Validate a FHIRPath expression syntax
///
/// # Arguments
/// * `expression` - The FHIRPath expression to validate
///
/// # Returns
/// A JSON string indicating whether the expression is valid
#[wasm_bindgen]
pub fn validate_fhirpath(expression: &str) -> String {
    // Try to parse the expression with an empty resource to check syntax
    let empty_resource = serde_json::Value::Object(serde_json::Map::new());

    match fhirpath_core::evaluate(expression, empty_resource) {
        Ok(_) => r#"{"valid": true}"#.to_string(),
        Err(e) => {
            format!(r#"{{"valid": false, "error": "{}"}}"#, e)
        }
    }
}

/// Get the FHIRPath specification version
#[wasm_bindgen]
pub fn get_fhirpath_version() -> String {
    fhirpath_core::FHIRPATH_SPEC_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_evaluate_simple_expression() {
        let resource = r#"{"resourceType": "Patient", "name": [{"given": ["John"], "family": "Doe"}]}"#;
        let result = evaluate_fhirpath("Patient.name.given", resource);
        assert!(result.contains("John"));
    }

    #[wasm_bindgen_test]
    fn test_validate_expression() {
        let result = validate_fhirpath("Patient.name");
        assert!(result.contains(r#""valid": true"#));
    }

    #[wasm_bindgen_test]
    fn test_invalid_expression() {
        let result = validate_fhirpath("Patient.name.invalid(");
        assert!(result.contains(r#""valid": false"#));
    }
}
