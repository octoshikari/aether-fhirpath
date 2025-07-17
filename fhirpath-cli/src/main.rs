// FHIRPath CLI
//
// Command-line interface for evaluating FHIRPath expressions against FHIR resources.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use fhirpath_core::evaluator::{evaluate_expression, evaluate_expression_streaming};
use fhirpath_core::model::FhirPathValue;
use fhirpath_core::lexer::tokenize;
use fhirpath_core::parser::parse;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fhirpath-cli")]
#[command(about = "Command-line interface for FHIRPath", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Evaluate a FHIRPath expression against a FHIR resource
    Eval {
        /// FHIRPath expression to evaluate
        expression: String,

        /// Path to FHIR resource JSON file
        #[arg(short, long)]
        resource: PathBuf,

        /// Output format (json, pretty)
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },

    /// Validate a FHIRPath expression syntax
    Validate {
        /// FHIRPath expression to validate
        expression: String,
    },
}

fn main() -> Result<()> {
    human_panic::setup_panic!();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Eval {
            expression,
            resource,
            format,
        } => {
            println!("{} {}", "Expression:".green().bold(), expression);
            println!("{} {}", "Resource:".green().bold(), resource.display());

            // Check file size to determine if we should use streaming mode
            const STREAMING_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB
            let metadata = fs::metadata(resource)
                .with_context(|| format!("Failed to get metadata for resource file: {}", resource.display()))?;

            let result = if metadata.len() > STREAMING_THRESHOLD {
                println!("{} Using streaming mode for large file ({} bytes)", "Info:".yellow().bold(), metadata.len());

                // Use streaming mode for large files
                let file = fs::File::open(resource)
                    .with_context(|| format!("Failed to open resource file: {}", resource.display()))?;

                evaluate_expression_streaming(expression, file)
                    .map_err(|e| anyhow::anyhow!("FHIRPath evaluation error: {}", e))
            } else {
                // Use regular mode for smaller files
                let resource_content = fs::read_to_string(resource)
                    .with_context(|| format!("Failed to read resource file: {}", resource.display()))?;

                // Parse the resource as JSON
                let resource_json: serde_json::Value = serde_json::from_str(&resource_content)
                    .with_context(|| "Failed to parse resource as JSON")?;

                evaluate_expression(expression, resource_json)
                    .map_err(|e| anyhow::anyhow!("FHIRPath evaluation error: {}", e))
            };

            match result {
                Ok(value) => {
                    println!("{} ", "Result:".green().bold());
                    match format.as_str() {
                        "json" => {
                            match format_as_json(&value) {
                                Ok(json_str) => println!("{}", json_str),
                                Err(e) => println!("{} Failed to format as JSON: {}", "Error:".red().bold(), e),
                            }
                        },
                        "pretty" | _ => {
                            println!("{}", format_as_pretty(&value));
                        }
                    }
                },
                Err(error) => {
                    println!("{} {}", "Error:".red().bold(), error);
                }
            }

            Ok(())
        }
        Commands::Validate { expression } => {
            println!("{} {}", "Validating:".green().bold(), expression);

            // Validate the expression by attempting to tokenize and parse it
            match validate_expression(expression) {
                Ok(()) => {
                    println!("{} {}", "Result:".green().bold(), "Valid FHIRPath expression");
                },
                Err(error) => {
                    println!("{} {}", "Result:".red().bold(), format!("Invalid: {}", error));
                }
            }

            Ok(())
        }
    }
}

/// Validate a FHIRPath expression syntax
fn validate_expression(expression: &str) -> Result<(), String> {
    // First, try to tokenize the expression
    let tokens = match tokenize(expression) {
        Ok(tokens) => tokens,
        Err(error) => return Err(error.to_string()),
    };

    // Then, try to parse the tokens
    match parse(&tokens) {
        Ok(_) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// Format FhirPathValue as JSON string
fn format_as_json(value: &FhirPathValue) -> Result<String, serde_json::Error> {
    match value {
        FhirPathValue::Empty => Ok("null".to_string()),
        FhirPathValue::Boolean(b) => serde_json::to_string_pretty(b),
        FhirPathValue::Integer(i) => serde_json::to_string_pretty(i),
        FhirPathValue::Decimal(d) => serde_json::to_string_pretty(d),
        FhirPathValue::String(s) => serde_json::to_string_pretty(s),
        FhirPathValue::Date(d) => serde_json::to_string_pretty(d),
        FhirPathValue::DateTime(dt) => serde_json::to_string_pretty(dt),
        FhirPathValue::Time(t) => serde_json::to_string_pretty(t),
        FhirPathValue::Quantity { value, unit } => {
            let quantity = serde_json::json!({
                "value": value,
                "unit": unit
            });
            serde_json::to_string_pretty(&quantity)
        },
        FhirPathValue::Collection(items) => {
            let json_items: Result<Vec<serde_json::Value>, _> = items.iter()
                .map(|item| value_to_json(item))
                .collect();
            match json_items {
                Ok(items) => serde_json::to_string_pretty(&items),
                Err(e) => Err(e),
            }
        },
        FhirPathValue::Resource(resource) => {
            serde_json::to_string_pretty(&resource.to_json())
        }
    }
}

/// Format FhirPathValue as pretty-printed string
fn format_as_pretty(value: &FhirPathValue) -> String {
    match value {
        FhirPathValue::Empty => "{}".to_string(),
        FhirPathValue::Boolean(b) => b.to_string(),
        FhirPathValue::Integer(i) => i.to_string(),
        FhirPathValue::Decimal(d) => d.to_string(),
        FhirPathValue::String(s) => format!("\"{}\"", s),
        FhirPathValue::Date(d) => format!("@{}", d),
        FhirPathValue::DateTime(dt) => format!("@{}", dt),
        FhirPathValue::Time(t) => format!("@{}", t),
        FhirPathValue::Quantity { value, unit } => {
            format!("{} '{}'", value, unit)
        },
        FhirPathValue::Collection(items) => {
            if items.is_empty() {
                "{}".to_string()
            } else if items.len() == 1 {
                format_as_pretty(&items[0])
            } else {
                let formatted_items: Vec<String> = items.iter()
                    .map(|item| format_as_pretty(item))
                    .collect();
                format!("[{}]", formatted_items.join(", "))
            }
        },
        FhirPathValue::Resource(resource) => {
            match serde_json::to_string_pretty(&resource.to_json()) {
                Ok(json) => json,
                Err(_) => format!("{:?}", resource),
            }
        }
    }
}

/// Convert FhirPathValue to serde_json::Value
fn value_to_json(value: &FhirPathValue) -> Result<serde_json::Value, serde_json::Error> {
    match value {
        FhirPathValue::Empty => Ok(serde_json::Value::Null),
        FhirPathValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        FhirPathValue::Integer(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
        FhirPathValue::Decimal(d) => {
            match serde_json::Number::from_f64(*d) {
                Some(num) => Ok(serde_json::Value::Number(num)),
                None => Ok(serde_json::Value::Null),
            }
        },
        FhirPathValue::String(s) => Ok(serde_json::Value::String(s.clone())),
        FhirPathValue::Date(d) => Ok(serde_json::Value::String(d.clone())),
        FhirPathValue::DateTime(dt) => Ok(serde_json::Value::String(dt.clone())),
        FhirPathValue::Time(t) => Ok(serde_json::Value::String(t.clone())),
        FhirPathValue::Quantity { value, unit } => {
            Ok(serde_json::json!({
                "value": value,
                "unit": unit
            }))
        },
        FhirPathValue::Collection(items) => {
            let json_items: Result<Vec<serde_json::Value>, _> = items.iter()
                .map(|item| value_to_json(item))
                .collect();
            match json_items {
                Ok(items) => Ok(serde_json::Value::Array(items)),
                Err(e) => Err(e),
            }
        },
        FhirPathValue::Resource(resource) => {
            Ok(resource.to_json())
        }
    }
}
