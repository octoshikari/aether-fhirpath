---
title: CLI Tool Usage
description: Complete guide to using the Aether FHIRPath command-line interface
---

The Aether FHIRPath CLI tool provides a convenient way to evaluate and validate FHIRPath expressions from the command line.

## Installation

First, make sure you have the CLI tool installed:

```bash
cargo install --path fhirpath-cli
```

## Commands

The CLI provides two main commands:

### `eval` - Evaluate FHIRPath Expressions

Evaluate FHIRPath expressions against FHIR resources.

#### Basic Syntax

```bash
aether-fhirpath eval <EXPRESSION> <RESOURCE_FILE> [OPTIONS]
```

#### Examples

```bash
# Evaluate a simple expression
aether-fhirpath eval "Patient.name.given" patient.json

# Evaluate with pretty formatting (default)
aether-fhirpath eval "name.family" patient.json --format pretty

# Evaluate with JSON formatting
aether-fhirpath eval "name.family" patient.json --format json

# Evaluate complex expressions
aether-fhirpath eval "name.where(use = 'official').family" patient.json
```

#### Options

- `--format <FORMAT>`: Output format (`pretty` or `json`)
  - `pretty`: Human-readable format (default)
  - `json`: JSON format for programmatic use

#### Working with Different Resource Types

```bash
# Patient resource
aether-fhirpath eval "name.given" patient.json

# Observation resource
aether-fhirpath eval "code.coding.system" observation.json

# Bundle resource
aether-fhirpath eval "entry.resource.resourceType" bundle.json

# Medication resource
aether-fhirpath eval "ingredient.item.display" medication.json
```

### `validate` - Validate FHIRPath Expressions

Check if FHIRPath expressions are syntactically valid.

#### Basic Syntax

```bash
aether-fhirpath validate <EXPRESSION>
```

#### Examples

```bash
# Valid expression
aether-fhirpath validate "Patient.name.family"
# Output: Expression is valid

# Invalid expression
aether-fhirpath validate "Patient.invalid..syntax"
# Output: Syntax error: unexpected token at position 15
```

## Common Use Cases

### Data Extraction

Extract specific data from FHIR resources:

```bash
# Get patient demographics
aether-fhirpath eval "name.family" patient.json
aether-fhirpath eval "gender" patient.json
aether-fhirpath eval "birthDate" patient.json

# Get observation values
aether-fhirpath eval "valueQuantity.value" observation.json
aether-fhirpath eval "valueQuantity.unit" observation.json
```

### Data Filtering

Filter data based on conditions:

```bash
# Get official names only
aether-fhirpath eval "name.where(use = 'official')" patient.json

# Get active medications
aether-fhirpath eval "entry.resource.where(status = 'active')" medication-statement.json
```

### Data Validation

Validate data presence and format:

```bash
# Check if required fields exist
aether-fhirpath eval "name.exists()" patient.json
aether-fhirpath eval "identifier.exists()" patient.json

# Validate data types
aether-fhirpath eval "birthDate.matches('[0-9]{4}-[0-9]{2}-[0-9]{2}')" patient.json
```

## Output Examples

### Pretty Format (Default)

```bash
$ aether-fhirpath eval "name.given" patient.json
["John", "Michael"]
```

### JSON Format

```bash
$ aether-fhirpath eval "name.given" patient.json --format json
["John","Michael"]
```

## Error Handling

The CLI provides detailed error messages for common issues:

### Syntax Errors

```bash
$ aether-fhirpath validate "Patient..name"
Error: Syntax error at position 8: unexpected token '..'
```

### File Not Found

```bash
$ aether-fhirpath eval "name" nonexistent.json
Error: File not found: nonexistent.json
```

### Invalid JSON

```bash
$ aether-fhirpath eval "name" invalid.json
Error: Invalid JSON in file: invalid.json
Line 3: unexpected character '{'
```

## Tips and Best Practices

1. **Use quotes**: Always wrap FHIRPath expressions in quotes to prevent shell interpretation
2. **Test expressions**: Use the `validate` command to check syntax before evaluation
3. **Format output**: Use `--format json` for programmatic processing
4. **Complex expressions**: Break down complex expressions into simpler parts for debugging

## Integration with Scripts

The CLI tool can be easily integrated into shell scripts:

```bash
#!/bin/bash

# Extract patient data
FAMILY_NAME=$(aether-fhirpath eval "name.family" patient.json --format json | jq -r '.[0]')
GIVEN_NAME=$(aether-fhirpath eval "name.given[0]" patient.json --format json | jq -r '.')

echo "Patient: $GIVEN_NAME $FAMILY_NAME"
```

## Next Steps

- Explore more [usage examples](/examples/usage-examples/)
- Learn about [Rust library usage](/usage/rust/)
- Check out [Node.js integration](/usage/nodejs/)
