# Aether FHIRPath Playground

A standalone interactive playground for testing FHIRPath expressions powered by WebAssembly.

## Features

- **Interactive FHIRPath Evaluation**: Test FHIRPath expressions against FHIR resources
- **Expression Validation**: Validate FHIRPath syntax without evaluation
- **WebAssembly Powered**: High-performance evaluation using Rust-compiled WASM
- **Quick Examples**: Pre-built example expressions for common use cases
- **Real-time Results**: Instant feedback with formatted JSON output

## How to Run

### Option 1: Simple HTTP Server (Python)
```bash
cd playground
python3 -m http.server 8080
```

### Option 2: Node.js HTTP Server
```bash
cd playground
npx http-server -p 8080
```

### Option 3: Any Static File Server
Since this is a static web application, you can serve it with any HTTP server.

## Usage

1. Open your browser and navigate to `http://localhost:8080`
2. Enter or modify the FHIR resource JSON in the left panel
3. Enter a FHIRPath expression in the expression input field
4. Click "Evaluate" to run the expression or "Validate" to check syntax
5. View results in the right panel

## Quick Examples

The playground includes several pre-built examples:
- `Patient.name.given` - Extract given names
- `Patient.name.family` - Extract family name
- `Patient.gender` - Get patient gender
- `Patient.birthDate` - Get birth date
- `Patient.address.city` - Extract city from address
- `Patient.telecom.where(system='phone').value` - Get phone numbers

## Files

- `index.html` - Main playground interface
- `app.js` - JavaScript application logic
- `styles.css` - Styling and responsive design
- `pkg/` - WebAssembly module and bindings
  - `fhirpath_wasm.js` - JavaScript bindings
  - `fhirpath_wasm_bg.wasm` - WebAssembly binary

## Integration

This playground is linked from the main documentation site and can be accessed via:
- Sidebar navigation under "Examples"
- Header social links (external icon)

## Development

The playground uses ES6 modules and requires a web server to run due to CORS restrictions with local file access.
