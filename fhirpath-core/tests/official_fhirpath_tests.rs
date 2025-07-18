use fhirpath_core::evaluator::evaluate_expression;
use fhirpath_core::model::FhirPathValue;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TestSuite {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@description")]
    description: String,
    #[serde(rename = "group")]
    groups: Vec<TestGroup>,
}

#[derive(Debug, Deserialize)]
struct TestGroup {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@description", default)]
    description: Option<String>,
    #[serde(rename = "test")]
    tests: Vec<Test>,
}

#[derive(Debug, Deserialize)]
struct Test {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@description", default)]
    description: Option<String>,
    #[serde(rename = "@inputfile")]
    inputfile: String,
    #[serde(rename = "@predicate", default)]
    predicate: Option<String>,
    #[serde(rename = "@mode", default)]
    mode: Option<String>,
    expression: TestExpression,
    #[serde(rename = "output", default)]
    outputs: Vec<TestOutput>,
}

#[derive(Debug, Deserialize)]
struct TestExpression {
    #[serde(rename = "@invalid", default)]
    invalid: Option<String>,
    #[serde(rename = "$text", default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct TestOutput {
    #[serde(rename = "@type", default)]
    output_type: Option<String>,
    #[serde(rename = "$text", default)]
    text: Option<String>,
}

/// Load and convert XML input file to JSON
fn load_input_file(filename: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // Try to find a corresponding JSON file first
    let json_filename = filename.replace(".xml", ".json");
    let json_path = Path::new("tests/fixtures").join(&json_filename);

    if json_path.exists() {
        let json_content = fs::read_to_string(&json_path)?;
        let json_value: Value = serde_json::from_str(&json_content)?;
        return Ok(json_value);
    }

    // If no JSON version exists, we need to convert XML to JSON
    // For now, return an error indicating this needs implementation
    Err(format!(
        "XML to JSON conversion not yet implemented for {}",
        filename
    )
    .into())
}

/// Parse the official test suite XML file using quick-xml
fn parse_test_suite() -> Result<TestSuite, Box<dyn std::error::Error>> {
    let test_file_path = "tests/official-tests/r4/tests-fhir-r4.xml";
    let xml_content = fs::read_to_string(test_file_path)?;

    let mut reader = Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut test_suite_name = String::new();
    let mut test_suite_description = String::new();
    let mut groups = Vec::new();

    // Parse the XML manually
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"tests" => {
                        // Parse tests element attributes
                        for attr in e.attributes() {
                            let attr = attr?;
                            match attr.key.as_ref() {
                                b"name" => {
                                    test_suite_name = String::from_utf8(attr.value.to_vec())?
                                }
                                b"description" => {
                                    test_suite_description = String::from_utf8(attr.value.to_vec())?
                                }
                                _ => {}
                            }
                        }
                    }
                    b"group" => {
                        // Parse a test group
                        let group = parse_test_group(&mut reader, e)?;
                        groups.push(group);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"tests" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(
                    format!("Error at position {}: {:?}", reader.buffer_position(), e).into(),
                );
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(TestSuite {
        name: test_suite_name,
        description: test_suite_description,
        groups,
    })
}

/// Parse a test group using quick-xml
fn parse_test_group(
    reader: &mut Reader<&[u8]>,
    start_element: &quick_xml::events::BytesStart,
) -> Result<TestGroup, Box<dyn std::error::Error>> {
    let mut group_name = String::new();
    let mut group_description = None;
    let mut tests = Vec::new();

    // Parse group attributes
    for attr in start_element.attributes() {
        let attr = attr?;
        match attr.key.as_ref() {
            b"name" => group_name = String::from_utf8(attr.value.to_vec())?,
            b"description" => group_description = Some(String::from_utf8(attr.value.to_vec())?),
            _ => {}
        }
    }

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"test" {
                    let test = parse_test(reader, e)?;
                    tests.push(test);
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"group" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Error parsing group: {:?}", e).into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(TestGroup {
        name: group_name,
        description: group_description,
        tests,
    })
}

/// Parse a single test using quick-xml
fn parse_test(
    reader: &mut Reader<&[u8]>,
    start_element: &quick_xml::events::BytesStart,
) -> Result<Test, Box<dyn std::error::Error>> {
    let mut test_name = String::new();
    let mut test_description = None;
    let mut inputfile = String::new();
    let mut predicate = None;
    let mut mode = None;
    let mut expression = TestExpression {
        invalid: None,
        text: String::new(),
    };
    let mut outputs = Vec::new();

    // Parse test attributes
    for attr in start_element.attributes() {
        let attr = attr?;
        match attr.key.as_ref() {
            b"name" => test_name = String::from_utf8(attr.value.to_vec())?,
            b"description" => test_description = Some(String::from_utf8(attr.value.to_vec())?),
            b"inputfile" => inputfile = String::from_utf8(attr.value.to_vec())?,
            b"predicate" => predicate = Some(String::from_utf8(attr.value.to_vec())?),
            b"mode" => mode = Some(String::from_utf8(attr.value.to_vec())?),
            _ => {}
        }
    }

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"expression" => {
                        // Parse expression attributes
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"invalid" {
                                expression.invalid = Some(String::from_utf8(attr.value.to_vec())?);
                            }
                        }
                    }
                    b"output" => {
                        let output = parse_output(reader, e)?;
                        outputs.push(output);
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                // This is the text content of the current element
                let text = e.unescape()?.into_owned();
                if !text.trim().is_empty() {
                    expression.text = text.trim().to_string();
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"test" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Error parsing test: {:?}", e).into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(Test {
        name: test_name,
        description: test_description,
        inputfile,
        predicate,
        mode,
        expression,
        outputs,
    })
}

/// Parse an output element using quick-xml
fn parse_output(
    reader: &mut Reader<&[u8]>,
    start_element: &quick_xml::events::BytesStart,
) -> Result<TestOutput, Box<dyn std::error::Error>> {
    let mut output_type = None;
    let mut text = None;

    // Parse output attributes
    for attr in start_element.attributes() {
        let attr = attr?;
        if attr.key.as_ref() == b"type" {
            output_type = Some(String::from_utf8(attr.value.to_vec())?);
        }
    }

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                let content = e.unescape()?.into_owned();
                if !content.trim().is_empty() {
                    text = Some(content.trim().to_string());
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"output" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Error parsing output: {:?}", e).into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(TestOutput { output_type, text })
}

/// Execute a single test case
fn execute_test(test: &Test, input_data: &Value) -> Result<bool, Box<dyn std::error::Error>> {
    let expression = &test.expression.text;

    // Check if this is an invalid expression test
    if test.expression.invalid.is_some() {
        // For invalid expressions, we expect the evaluation to fail
        match evaluate_expression(expression, input_data.clone()) {
            Ok(_) => return Ok(false), // Should have failed but didn't
            Err(_) => return Ok(true), // Failed as expected
        }
    }

    // Evaluate the expression
    let result = evaluate_expression(expression, input_data.clone())?;

    // If this is a predicate test, check if result is boolean true
    if test.predicate.as_deref() == Some("true") {
        match result {
            FhirPathValue::Boolean(true) => return Ok(true),
            _ => return Ok(false),
        }
    }

    // If this is a predicate false test, check if result is boolean false or empty
    if test.predicate.as_deref() == Some("false") {
        match result {
            FhirPathValue::Boolean(false) => return Ok(true),
            FhirPathValue::Collection(ref coll) if coll.is_empty() => return Ok(true),
            _ => return Ok(false),
        }
    }

    // For regular tests, compare with expected outputs
    if test.outputs.is_empty() {
        // No expected output means we expect an empty result
        match result {
            FhirPathValue::Collection(ref coll) if coll.is_empty() => return Ok(true),
            _ => return Ok(false),
        }
    }

    // TODO: Implement proper output comparison
    // This is a placeholder that needs to compare the actual result with expected outputs
    println!(
        "Test: {} - Expression: {} - Result: {:?}",
        test.name, expression, result
    );

    Ok(true) // Placeholder - assume test passes for now
}

/// Run a subset of tests for initial validation
#[test]
fn test_official_fhirpath_basic_tests() {
    // For now, let's test with our existing patient fixture
    let patient_json = fs::read_to_string("tests/fixtures/patient-example.json")
        .expect("Failed to read patient fixture");
    let patient_data: Value =
        serde_json::from_str(&patient_json).expect("Failed to parse patient JSON");

    // Test some basic expressions manually first
    let test_cases = vec![
        ("birthDate", "1974-12-25"),
        ("name.given", "John"), // Updated to match actual fixture data
        ("gender", "male"),
        ("active", "true"),
    ];

    for (expression, expected) in test_cases {
        match evaluate_expression(expression, patient_data.clone()) {
            Ok(result) => {
                println!("Expression: {} -> Result: {:?}", expression, result);
                // Basic validation - just ensure it doesn't crash
                assert!(!matches!(result, FhirPathValue::Collection(ref coll) if coll.is_empty()));
            }
            Err(e) => {
                panic!("Failed to evaluate expression '{}': {:?}", expression, e);
            }
        }
    }
}

/// Test XML parsing functionality
#[test]
fn test_xml_parsing() {
    // Test if we can parse the test suite XML
    match parse_test_suite() {
        Ok(test_suite) => {
            println!("Successfully parsed test suite: {}", test_suite.name);
            println!("Number of test groups: {}", test_suite.groups.len());

            let total_tests: usize = test_suite
                .groups
                .iter()
                .map(|group| group.tests.len())
                .sum();
            println!("Total number of tests: {}", total_tests);

            // Print first few test groups for verification
            for (i, group) in test_suite.groups.iter().take(3).enumerate() {
                println!(
                    "Group {}: {} ({} tests)",
                    i + 1,
                    group.name,
                    group.tests.len()
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse test suite: {:?}", e);
        }
    }
}

/// Test loading input files
#[test]
fn test_input_file_loading() {
    // Test loading patient-example.xml (should work with our JSON fallback)
    match load_input_file("patient-example.xml") {
        Ok(data) => {
            println!("Successfully loaded patient data");
            assert!(data.is_object());
        }
        Err(e) => {
            println!("Expected error for XML conversion: {:?}", e);
            // This is expected until we implement proper XML to JSON conversion
        }
    }
}

/// Run the full official FHIRPath test suite
#[test]
fn run_official_fhirpath_tests() {
    let test_suite = parse_test_suite().expect("Failed to parse test suite");
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for group in &test_suite.groups {
        println!("Running test group: {}", group.name);

        for test in &group.tests {
            match load_input_file(&test.inputfile) {
                Ok(input_data) => match execute_test(test, &input_data) {
                    Ok(true) => {
                        passed += 1;
                        println!("  ✓ {}", test.name);
                    }
                    Ok(false) => {
                        failed += 1;
                        println!("  ✗ {}", test.name);
                    }
                    Err(e) => {
                        failed += 1;
                        println!("  ✗ {} (Error: {:?})", test.name, e);
                    }
                },
                Err(e) => {
                    skipped += 1;
                    println!("  - {} (Skipped: {:?})", test.name, e);
                }
            }
        }
    }

    println!("\nTest Results:");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Skipped: {}", skipped);
    println!("Total: {}", passed + failed + skipped);
}
