/* tslint:disable */
/* eslint-disable */

/**
 * FHIRPath engine for evaluating FHIRPath expressions against FHIR resources
 */
export class FhirPathEngine {
  /**
   * Creates a new FHIRPath engine instance
   */
  constructor();

  /**
   * Evaluates a FHIRPath expression against a FHIR resource
   * @param expression - The FHIRPath expression to evaluate
   * @param resource - The FHIR resource JSON string to evaluate against
   * @returns The evaluation result as a JSON string
   */
  evaluate(expression: string, resource: string): string;

  /**
   * Validates a FHIRPath expression syntax
   * @param expression - The FHIRPath expression to validate
   * @returns True if the expression is valid, false otherwise
   */
  validate(expression: string): boolean;

  /**
   * Returns the version of the FHIRPath engine
   * @returns The version string
   */
  version(): string;
}

/**
 * Returns information about the FHIRPath engine
 * @returns Information about the FHIRPath engine
 */
export function getEngineInfo(): string;
