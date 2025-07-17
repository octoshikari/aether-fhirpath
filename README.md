# Aether FHIRPath

A high-performance FHIRPath implementation in Rust with multiple language bindings.

## Overview

Aether FHIRPath is a comprehensive implementation of the [FHIRPath specification](http://hl7.org/fhirpath/) written in Rust. FHIRPath is a path-based navigation and extraction language for FHIR (Fast Healthcare Interoperability Resources) data.

This project provides:
- A fast, memory-safe core engine written in Rust
- A command-line interface for FHIRPath evaluation
- Node.js bindings for JavaScript/TypeScript integration

## Features

- ✅ **High Performance**: Written in Rust for maximum speed and memory safety
- ✅ **Multiple Interfaces**: CLI tool, Rust library, and Node.js bindings
- ✅ **FHIRPath Compliance**: Implements the official FHIRPath specification
- ✅ **Expression Validation**: Syntax validation for FHIRPath expressions
- ✅ **Multiple Output Formats**: JSON and pretty-printed output
- ✅ **Streaming Support**: Efficient processing of large FHIR resources

## Components

### fhirpath-core
The core FHIRPath engine that provides:
- Lexical analysis and tokenization
- Expression parsing using nom parser combinators
- Expression evaluation against FHIR resources
- Comprehensive error handling

### fhirpath-cli
A command-line interface that allows you to:
- Evaluate FHIRPath expressions against FHIR resources
- Validate FHIRPath expression syntax
- Output results in multiple formats

### fhirpath-node
Node.js bindings that enable:
- JavaScript/TypeScript integration
- Native performance in Node.js applications
- Seamless JSON handling

## Installation

### CLI Tool

```bash
# Install from source
git clone https://github.com/aether-fhirpath/fhirpath-rust
cd aether-fhirpath
cargo install --path fhirpath-cli
```

### Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
fhirpath-core = "0.1.0"
```

### Node.js Package

```bash
npm install fhirpath-node
```

## Usage

### Command Line Interface

The CLI provides two main commands:

#### Evaluate FHIRPath expressions

```bash
# Evaluate an expression against a FHIR resource
aether-fhirpath eval "Patient.name.given" patient.json

# Specify output format
aether-fhirpath eval "Patient.name.given" patient.json --format json
aether-fhirpath eval "Patient.name.given" patient.json --format pretty
```

#### Validate FHIRPath expressions

```bash
# Check if an expression is syntactically valid
aether-fhirpath validate "Patient.name.given"
aether-fhirpath validate "Patient.invalid..syntax"
```

### Rust Library

```rust
use fhirpath_core::evaluator::evaluate_expression;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fhir_resource: Value = serde_json::from_str(r#"
    {
        "resourceType": "Patient",
        "name": [
            {
                "given": ["John", "Doe"]
            }
        ]
    }
    "#)?;
    
    let expression = "Patient.name.given";
    let result = evaluate_expression(expression, &fhir_resource)?;
    
    println!("Result: {:?}", result);
    Ok(())
}
```

### Node.js

```javascript
const { evaluateExpression } = require('fhirpath-node');

const fhirResource = {
    resourceType: "Patient",
    name: [
        {
            given: ["John", "Doe"]
        }
    ]
};

const expression = "Patient.name.given";
const result = evaluateExpression(expression, fhirResource);
console.log('Result:', result);
```

## Development

### Prerequisites

- Rust 1.70+ with 2024 edition support
- Node.js 20+ (for Node.js bindings)
- Cargo

### Building from Source

```bash
# Clone the repository
git clone https://github.com/aether-fhirpath/fhirpath-rust
cd aether-fhirpath

# Build all components
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Building Node.js Bindings

```bash
cd fhirpath-node
npm install
npm run build
npm test
```

### Project Structure

```
aether-fhirpath/
├── fhirpath-core/          # Core FHIRPath engine
│   ├── src/
│   │   ├── lexer.rs        # Tokenization
│   │   ├── parser.rs       # Expression parsing
│   │   ├── evaluator.rs    # Expression evaluation
│   │   └── model.rs        # Data models
│   └── tests/              # Core tests
├── fhirpath-cli/           # Command-line interface
│   └── src/main.rs         # CLI implementation
├── fhirpath-node/          # Node.js bindings
│   └── src/lib.rs          # NAPI bindings
└── docs/                   # Documentation
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific component
cargo test -p fhirpath-core
cargo test -p fhirpath-cli

# Run with debug output
cargo test -- --nocapture
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for your changes
5. Ensure all tests pass (`cargo test`)
6. Run clippy for linting (`cargo clippy`)
7. Format code (`cargo fmt`)
8. Commit your changes (`git commit -m 'Add amazing feature'`)
9. Push to the branch (`git push origin feature/amazing-feature`)
10. Open a Pull Request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [HL7 FHIR](https://www.hl7.org/fhir/) for the FHIRPath specification
- The Rust community for excellent tooling and libraries
- Contributors and maintainers of this project

## Support

- 📖 [Documentation](https://github.com/octoshikari/aether-fhirpath/docs)
- 🐛 [Issue Tracker](https://github.com/octoshikari/aether-fhirpath/issues)
- 💬 [Discussions](https://github.com/octoshikari/aether-fhirpath/discussions)
