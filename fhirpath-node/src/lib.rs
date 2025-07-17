// FHIRPath Node.js Bindings
//
// This crate provides Node.js bindings for the FHIRPath engine.

#[macro_use]
extern crate napi_derive;

use napi::{Error, Result};
use fhirpath_core;

#[napi]
pub struct FhirPathEngine {
    // Internal state if needed
}

#[napi]
impl FhirPathEngine {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    /// Evaluates a FHIRPath expression against a FHIR resource
    #[napi]
    pub fn evaluate(&self, expression: String, resource: String) -> Result<String> {
        // Parse the resource as JSON
        let resource_json = match serde_json::from_str::<serde_json::Value>(&resource) {
            Ok(json) => json,
            Err(err) => return Err(Error::from_reason(format!("Failed to parse resource as JSON: {}", err))),
        };

        // Evaluate the expression using the core FHIRPath engine
        let result = match fhirpath_core::evaluate(&expression, resource_json) {
            Ok(value) => serde_json::to_string(&value)
                .map_err(|err| Error::from_reason(format!("Failed to serialize result: {}", err)))?,
            Err(err) => return Err(Error::from_reason(format!("FHIRPath evaluation error: {}", err))),
        };

        Ok(result)
    }

    /// Validates a FHIRPath expression syntax
    #[napi]
    pub fn validate(&self, expression: String) -> Result<bool> {
        // Tokenize the expression
        let tokens = match fhirpath_core::lexer::tokenize(&expression) {
            Ok(tokens) => tokens,
            Err(_) => {
                // Return false for syntax errors in tokenization
                return Ok(false);
            }
        };

        // Parse the tokens
        match fhirpath_core::parser::parse(&tokens) {
            Ok(_) => Ok(true),  // Parsing succeeded, expression is valid
            Err(_) => Ok(false) // Parsing failed, expression is invalid
        }
    }

    /// Returns the version of the FHIRPath engine
    #[napi]
    pub fn version(&self) -> String {
        format!("FHIRPath Engine v{} (spec: {})",
                env!("CARGO_PKG_VERSION"),
                fhirpath_core::FHIRPATH_SPEC_VERSION)
    }
}

#[napi]
pub fn get_engine_info() -> String {
    format!("FHIRPath Rust Engine v{} (Node.js bindings)", env!("CARGO_PKG_VERSION"))
}
