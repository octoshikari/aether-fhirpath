import { FhirPathEngine, getEngineInfo } from '../index';

// Sample FHIR resource for testing
const patientResource = JSON.stringify({
  resourceType: 'Patient',
  id: 'example',
  name: [
    {
      use: 'official',
      family: 'Smith',
      given: ['John', 'Jacob']
    }
  ],
  gender: 'male',
  birthDate: '1974-12-25'
});

describe('FHIRPath Node.js Bindings', () => {
  let engine: FhirPathEngine;

  beforeEach(() => {
    engine = new FhirPathEngine();
  });

  test('should create an engine instance', () => {
    expect(engine).toBeInstanceOf(FhirPathEngine);
  });

  test('should return engine info', () => {
    const info = getEngineInfo();
    expect(info).toContain('FHIRPath Rust Engine');
  });

  test('should return engine version', () => {
    const version = engine.version();
    expect(version).toContain('FHIRPath Engine v');
  });

  test('should validate a valid FHIRPath expression', () => {
    const isValid = engine.validate('Patient.name.given');
    expect(isValid).toBe(true);
  });

  test('should invalidate an invalid FHIRPath expression', () => {
    const isValid = engine.validate('Patient.name.[');
    expect(isValid).toBe(false);
  });

  test('should evaluate a FHIRPath expression', () => {
    const result = engine.evaluate('Patient.name.given', patientResource);
    // Parse the result as JSON to check its structure
    const parsedResult = JSON.parse(result);
    // The result should be an array with two elements: "John" and "Jacob"
    expect(Array.isArray(parsedResult)).toBe(true);
    expect(parsedResult).toContain('John');
    expect(parsedResult).toContain('Jacob');
  });
});
