---
title: Quick Start
description: Get up and running with Aether FHIRPath in minutes
---

This guide will help you get started with Aether FHIRPath quickly using the CLI tool.

## Prerequisites

Make sure you have [installed](/getting-started/installation/) the CLI tool:

```bash
cargo install --path fhirpath-cli
```

## Basic Usage

### Evaluate FHIRPath Expressions

The most common use case is evaluating FHIRPath expressions against FHIR resources.

```bash
# Evaluate an expression against a FHIR resource
aether-fhirpath eval "Patient.name.given" patient.json

# Specify output format
aether-fhirpath eval "Patient.name.given" patient.json --format json
aether-fhirpath eval "Patient.name.given" patient.json --format pretty
```

### Validate FHIRPath Expressions

You can also validate FHIRPath expression syntax:

```bash
# Check if an expression is syntactically valid
aether-fhirpath validate "Patient.name.given"
aether-fhirpath validate "Patient.invalid..syntax"
```

## Example FHIR Resource

Create a sample `patient.json` file to test with:

```json
{
  "resourceType": "Patient",
  "id": "example",
  "name": [
    {
      "use": "official",
      "family": "Smith",
      "given": ["John", "Michael"]
    }
  ],
  "gender": "male",
  "birthDate": "1990-01-01"
}
```

## Common FHIRPath Expressions

Try these expressions with your sample patient:

```bash
# Get the resource type
aether-fhirpath eval "resourceType" patient.json

# Get the patient's family name
aether-fhirpath eval "name.family" patient.json

# Get all given names
aether-fhirpath eval "name.given" patient.json

# Get the first given name
aether-fhirpath eval "name.given[0]" patient.json

# Get the gender
aether-fhirpath eval "gender" patient.json

# Get names with official use
aether-fhirpath eval "name.where(use = 'official')" patient.json
```

## Output Formats

Aether FHIRPath supports different output formats:

### Pretty Format (Default)

```bash
aether-fhirpath eval "name.given" patient.json --format pretty
```

Output:
```
["John", "Michael"]
```

### JSON Format

```bash
aether-fhirpath eval "name.given" patient.json --format json
```

Output:
```json
["John", "Michael"]
```

## Next Steps

- Learn more about [CLI usage](/usage/cli/)
- Explore [usage examples](/examples/usage-examples/)
- Integrate with [Rust](/usage/rust/) or [Node.js](/usage/nodejs/)
- Read the [FHIRPath specification](http://hl7.org/fhirpath/)
