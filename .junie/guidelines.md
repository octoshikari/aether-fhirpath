# Aether FHIRPath Project Guidelines

## Project Overview

Aether FHIRPath is a high-performance implementation of the [FHIRPath specification](https://build.fhir.org/ig/HL7/FHIRPath/) written in Rust with multiple language bindings. FHIRPath is a path-based navigation and extraction language for FHIR (Fast Healthcare Interoperability Resources) data.

The project aims to provide:
- A fast, memory-safe core engine written in Rust
- Multiple interfaces (CLI, Rust library, Node.js bindings, WebAssembly)
- Full compliance with the official FHIRPath specification
- Comprehensive testing against the official test suite

### Key Resources
- [FHIRPath Specification](https://build.fhir.org/ig/HL7/FHIRPath/) - The official specification documentation
- [FHIRPath GitHub Repository](https://github.com/HL7/FHIRPath) - The official GitHub repository with test cases and examples

## Project Structure

The project is organized into several key components:

```
aether-fhirpath/
├── fhirpath-core/          # Core FHIRPath engine implementation
│   ├── src/
│   │   ├── lexer.rs        # Tokenization
│   │   ├── parser.rs       # AST parsing
│   │   ├── evaluator.rs    # Expression evaluation
│   │   ├── model.rs        # Data models
│   │   └── errors.rs       # Error types
│   ├── tests/              # Unit and integration tests
│   └── benches/            # Performance benchmarks
├── fhirpath-cli/           # Command-line interface
│   └── src/main.rs         # CLI implementation
├── fhirpath-node/          # Node.js bindings
│   ├── src/lib.rs          # NAPI bindings
│   └── test/               # Node.js tests
├── fhirpath-wasm/          # WebAssembly bindings
├── fhirpath-comparison/    # Comparison framework
│   ├── implementations/    # Different language implementations
│   ├── scripts/            # Comparison scripts
│   ├── results/            # Comparison results
│   └── visualization/      # Results visualization
├── docs/                   # Documentation
└── docs-site/             # Documentation website
```

### Key Components

- **Lexer**: Tokenizes FHIRPath expressions
- **Parser**: Builds Abstract Syntax Tree (AST)
- **Evaluator**: Executes expressions against FHIR resources
- **Model**: Defines FHIRPath value types
- **Bindings**: Integration layers for different platforms (Node.js, WebAssembly)
- **Comparison Framework**: Tools for comparing with other FHIRPath implementations

## Development Workflow

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Node.js 20+ (for Node.js bindings development)
- Git
- Pnpm as package manager for js/ts packages
- For js/ts/ packages use Biome linter/formatter instrad of Eslint and Prettier

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/aether-fhirpath.git
cd aether-fhirpath

# Install Rust dependencies
cargo build

# Install Node.js dependencies (for Node.js bindings)
cd fhirpath-node
npm install
cd ..
```

### Daily Development

```bash
# Start development session with continuous testing
cargo watch -x "test --workspace"

# In another terminal, run continuous linting
cargo watch -x "clippy --workspace"

# Before committing
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

### Pull Request Process

1. Create a feature branch from `main`
2. Make your changes following coding standards
3. Add tests for new functionality
4. Update documentation as needed
5. Run the full test suite
6. Submit a pull request

### Commit Messages

Use conventional commit format:

```
type(scope): description

Examples:
feat(parser): add support for new FHIRPath function
fix(evaluator): handle null values in path expressions
docs(readme): update installation instructions
test(lexer): add tests for edge cases
```

## Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with parallel execution (if nextest is installed)
cargo nextest run

# Run specific test
cargo test test_basic_property_access

# Run tests with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

### Test Organization

- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test component interactions
- **Conformance tests**: Test against FHIRPath specification
- **Performance tests**: Benchmark critical paths

### Test Compliance

The implementation is continuously tested against the official FHIRPath test suite. Current status:
- **179 tests passing** (25.2% compliance)
- **501 tests failing** (features not yet implemented)
- **31 tests skipped** (missing test data or dependencies)
- **711 total tests** in the official suite

## Build Instructions

### Building Core Components

```bash
# Build all components
cargo build --release

# Build specific component
cargo build --release -p fhirpath-core
cargo build --release -p fhirpath-cli
```

### Building Node.js Bindings

```bash
cd fhirpath-node
npm install
npm run build
```

### Building WebAssembly Bindings

```bash
cd fhirpath-wasm
wasm-pack build
```

### Building Documentation Site

```bash
cd docs-site
npm install
npm run dev  # For development
npm run build  # For production
```

## Code Style Guidelines

### Formatting

The project uses `rustfmt` for code formatting:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

The project uses `clippy` for linting:

```bash
# Run clippy for linting
cargo clippy --workspace --all-targets --all-features

# Fix clippy warnings where possible
cargo clippy --workspace --all-targets --all-features --fix
```

### Documentation

All public APIs should be documented with rustdoc comments:

```rust
/// Evaluates a FHIRPath expression against a FHIR resource.
///
/// # Arguments
///
/// * `expression` - The FHIRPath expression to evaluate
/// * `resource` - The FHIR resource as JSON
///
/// # Returns
///
/// Returns a `Result` containing the evaluation result or an error.
///
/// # Examples
///
/// ```rust
/// use fhirpath_core::evaluator::evaluate_expression;
/// use serde_json::json;
///
/// let resource = json!({"resourceType": "Patient", "id": "123"});
/// let result = evaluate_expression("id", resource)?;
/// ```
pub fn evaluate_expression(
    expression: &str,
    resource: serde_json::Value,
) -> Result<FhirPathValue, FhirPathError> {
    // Implementation
}
```

### Error Handling

- Use `Result<T, E>` for fallible operations
- Create specific error types for different failure modes
- Provide helpful error messages with context
- Use `anyhow` for application errors, custom types for library errors

## Security and Performance

### Security Guidelines

- Validate all external inputs
- Prevent resource exhaustion attacks
- Don't leak sensitive information in errors
- Keep dependencies updated and audited

### Performance Considerations

- Avoid unnecessary allocations in hot paths
- Use appropriate data structures for the use case
- Profile before optimizing - measure actual performance
- Consider memory usage as well as CPU performance

## Working with Junie

When working with Junie on this project:

1. **Run tests** to verify changes: `cargo test --workspace`
2. **Format code** before submitting: `cargo fmt --all`
3. **Run linting** to ensure code quality: `cargo clippy --workspace --all-targets --all-features`
4. **Build the project** to verify compilation: `cargo build --release`
5. **Check test compliance** to ensure no regressions in FHIRPath specification compliance

For complex changes, consider running the comparison framework to evaluate performance against other implementations:

```bash
cd fhirpath-comparison
python scripts/run-comparison.py
```
