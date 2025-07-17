#!/usr/bin/env node

/**
 * FHIRPath Node.js Integration Example
 *
 * This example demonstrates how to use the FHIRPath Rust engine
 * in a Node.js application for processing FHIR resources.
 */

const { FhirPathEngine, exists, getEngineInfo } = require('../fhirpath-node');
const fs = require('fs');
const path = require('path');

// Sample FHIR Patient resource
const samplePatient = {
    "resourceType": "Patient",
    "id": "example-patient",
    "meta": {
        "versionId": "1",
        "lastUpdated": "2023-01-01T12:00:00Z"
    },
    "identifier": [
        {
            "use": "usual",
            "type": {
                "coding": [
                    {
                        "system": "http://terminology.hl7.org/CodeSystem/v2-0203",
                        "code": "MR"
                    }
                ]
            },
            "system": "urn:oid:1.2.36.146.595.217.0.1",
            "value": "12345"
        }
    ],
    "active": true,
    "name": [
        {
            "use": "official",
            "family": "Smith",
            "given": ["John", "Jacob"]
        },
        {
            "use": "nickname",
            "given": ["Johnny"]
        }
    ],
    "telecom": [
        {
            "system": "phone",
            "value": "555-555-5555",
            "use": "home"
        },
        {
            "system": "email",
            "value": "john.smith@example.com",
            "use": "work"
        }
    ],
    "gender": "male",
    "birthDate": "1974-12-25",
    "address": [
        {
            "use": "home",
            "line": ["123 Main St"],
            "city": "Anytown",
            "state": "CA",
            "postalCode": "12345",
            "country": "USA"
        }
    ]
};

// Sample FHIR Observation resource
const sampleObservation = {
    "resourceType": "Observation",
    "id": "example-observation",
    "status": "final",
    "category": [
        {
            "coding": [
                {
                    "system": "http://terminology.hl7.org/CodeSystem/observation-category",
                    "code": "vital-signs"
                }
            ]
        }
    ],
    "code": {
        "coding": [
            {
                "system": "http://loinc.org",
                "code": "29463-7",
                "display": "Body Weight"
            }
        ]
    },
    "subject": {
        "reference": "Patient/example-patient"
    },
    "valueQuantity": {
        "value": 70.5,
        "unit": "kg",
        "system": "http://unitsofmeasure.org",
        "code": "kg"
    }
};

class FHIRPathDemo {
    constructor() {
        this.engine = new FhirPathEngine();
        console.log('🚀 FHIRPath Engine initialized');
        console.log('ℹ️  Engine info:', getEngineInfo());
        console.log('📋 Engine version:', this.engine.version());
        console.log('');
    }

    /**
     * Demonstrates basic property access
     */
    demonstrateBasicAccess() {
        console.log('=== Basic Property Access ===');

        const expressions = [
            'resourceType',
            'id',
            'gender',
            'birthDate',
            'active'
        ];

        expressions.forEach(expr => {
            try {
                const result = this.engine.evaluate(expr, JSON.stringify(samplePatient));
                const parsed = JSON.parse(result);
                console.log(`${expr.padEnd(15)} → ${JSON.stringify(parsed)}`);
            } catch (error) {
                console.error(`❌ Error evaluating '${expr}':`, error.message);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates path navigation and array access
     */
    demonstratePathNavigation() {
        console.log('=== Path Navigation ===');

        const expressions = [
            'name.family',
            'name.given',
            'name[0].family',
            'name[0].given[0]',
            'telecom.system',
            'telecom.value',
            'address.city',
            'identifier.value'
        ];

        expressions.forEach(expr => {
            try {
                const result = this.engine.evaluate(expr, JSON.stringify(samplePatient));
                const parsed = JSON.parse(result);
                console.log(`${expr.padEnd(20)} → ${JSON.stringify(parsed)}`);
            } catch (error) {
                console.error(`❌ Error evaluating '${expr}':`, error.message);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates complex nested expressions
     */
    demonstrateComplexExpressions() {
        console.log('=== Complex Expressions ===');

        const expressions = [
            'identifier.type.coding.code',
            'telecom.where(system = "email").value',
            'name.where(use = "official").family',
            'address.line[0]'
        ];

        expressions.forEach(expr => {
            try {
                const result = this.engine.evaluate(expr, JSON.stringify(samplePatient));
                const parsed = JSON.parse(result);
                console.log(`${expr.padEnd(35)} → ${JSON.stringify(parsed)}`);
            } catch (error) {
                console.error(`❌ Error evaluating '${expr}':`, error.message);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates asynchronous evaluation
     */
    async demonstrateAsyncEvaluation() {
        console.log('=== Asynchronous Evaluation ===');

        const expressions = [
            'resourceType',
            'name.family',
            'telecom.system'
        ];

        for (const expr of expressions) {
            try {
                const result = await this.engine.evaluateAsync(expr, JSON.stringify(samplePatient));
                const parsed = JSON.parse(result);
                console.log(`${expr.padEnd(15)} → ${JSON.stringify(parsed)} (async)`);
            } catch (error) {
                console.error(`❌ Async error evaluating '${expr}':`, error.message);
            }
        }
        console.log('');
    }

    /**
     * Demonstrates expression validation
     */
    demonstrateValidation() {
        console.log('=== Expression Validation ===');

        const expressions = [
            { expr: 'resourceType', expected: true },
            { expr: 'name.family', expected: true },
            { expr: 'invalid..expression', expected: false },
            { expr: 'name[', expected: false },
            { expr: 'telecom.where(system = "email")', expected: true }
        ];

        expressions.forEach(({ expr, expected }) => {
            const isValid = this.engine.validate(expr);
            const status = isValid ? '✅' : '❌';
            const match = isValid === expected ? '✓' : '✗';
            console.log(`${status} ${expr.padEnd(35)} → ${isValid} ${match}`);
        });
        console.log('');
    }

    /**
     * Demonstrates existence checking
     */
    demonstrateExistenceChecking() {
        console.log('=== Existence Checking ===');

        const expressions = [
            'name.where(use = "official")',
            'name.where(use = "maiden")',
            'telecom.where(system = "email")',
            'telecom.where(system = "fax")',
            'address'
        ];

        expressions.forEach(expr => {
            try {
                const hasResults = exists(expr, JSON.stringify(samplePatient));
                const status = hasResults ? '✅' : '❌';
                console.log(`${status} ${expr.padEnd(35)} → exists: ${hasResults}`);
            } catch (error) {
                console.error(`❌ Error checking existence for '${expr}':`, error.message);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates working with different resource types
     */
    demonstrateDifferentResourceTypes() {
        console.log('=== Different Resource Types ===');

        console.log('📊 Observation Resource:');
        const observationExpressions = [
            'resourceType',
            'status',
            'code.coding.display',
            'valueQuantity.value',
            'valueQuantity.unit'
        ];

        observationExpressions.forEach(expr => {
            try {
                const result = this.engine.evaluate(expr, JSON.stringify(sampleObservation));
                const parsed = JSON.parse(result);
                console.log(`  ${expr.padEnd(25)} → ${JSON.stringify(parsed)}`);
            } catch (error) {
                console.error(`  ❌ Error evaluating '${expr}':`, error.message);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates error handling patterns
     */
    demonstrateErrorHandling() {
        console.log('=== Error Handling ===');

        const invalidExpressions = [
            'invalid..syntax',
            'name[',
            'unknown.property.chain',
            ''
        ];

        invalidExpressions.forEach(expr => {
            try {
                const result = this.engine.evaluate(expr, JSON.stringify(samplePatient));
                console.log(`✅ Unexpected success for '${expr}': ${result}`);
            } catch (error) {
                console.log(`❌ Expected error for '${expr}': ${error.message}`);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates batch processing
     */
    async demonstrateBatchProcessing() {
        console.log('=== Batch Processing ===');

        const resources = [samplePatient, sampleObservation];
        const expression = 'resourceType';

        console.log('Processing multiple resources:');

        // Synchronous batch processing
        const syncResults = resources.map((resource, index) => {
            try {
                const result = this.engine.evaluate(expression, JSON.stringify(resource));
                const parsed = JSON.parse(result);
                return { index, resourceType: parsed[0], success: true };
            } catch (error) {
                return { index, error: error.message, success: false };
            }
        });

        syncResults.forEach(result => {
            if (result.success) {
                console.log(`  Resource ${result.index}: ${result.resourceType}`);
            } else {
                console.log(`  Resource ${result.index}: Error - ${result.error}`);
            }
        });

        // Asynchronous batch processing
        console.log('\nAsync batch processing:');
        const asyncResults = await Promise.all(
            resources.map(async (resource, index) => {
                try {
                    const result = await this.engine.evaluateAsync(expression, JSON.stringify(resource));
                    const parsed = JSON.parse(result);
                    return { index, resourceType: parsed[0], success: true };
                } catch (error) {
                    return { index, error: error.message, success: false };
                }
            })
        );

        asyncResults.forEach(result => {
            if (result.success) {
                console.log(`  Resource ${result.index}: ${result.resourceType} (async)`);
            } else {
                console.log(`  Resource ${result.index}: Error - ${result.error} (async)`);
            }
        });
        console.log('');
    }

    /**
     * Demonstrates performance considerations
     */
    async demonstratePerformance() {
        console.log('=== Performance Demonstration ===');

        const expression = 'name.family';
        const iterations = 1000;

        // Synchronous performance test
        console.log(`Running ${iterations} synchronous evaluations...`);
        const syncStart = Date.now();
        for (let i = 0; i < iterations; i++) {
            this.engine.evaluate(expression, JSON.stringify(samplePatient));
        }
        const syncEnd = Date.now();
        const syncTime = syncEnd - syncStart;
        console.log(`Sync: ${syncTime}ms total, ${(syncTime / iterations).toFixed(3)}ms per evaluation`);

        // Asynchronous performance test
        console.log(`Running ${iterations} asynchronous evaluations...`);
        const asyncStart = Date.now();
        const promises = [];
        for (let i = 0; i < iterations; i++) {
            promises.push(this.engine.evaluateAsync(expression, JSON.stringify(samplePatient)));
        }
        await Promise.all(promises);
        const asyncEnd = Date.now();
        const asyncTime = asyncEnd - asyncStart;
        console.log(`Async: ${asyncTime}ms total, ${(asyncTime / iterations).toFixed(3)}ms per evaluation`);
        console.log('');
    }

    /**
     * Runs all demonstrations
     */
    async runAllDemonstrations() {
        console.log('🔬 FHIRPath Node.js Integration Demo\n');

        this.demonstrateBasicAccess();
        this.demonstratePathNavigation();
        this.demonstrateComplexExpressions();
        await this.demonstrateAsyncEvaluation();
        this.demonstrateValidation();
        this.demonstrateExistenceChecking();
        this.demonstrateDifferentResourceTypes();
        this.demonstrateErrorHandling();
        await this.demonstrateBatchProcessing();
        await this.demonstratePerformance();

        console.log('✅ All demonstrations completed successfully!');
    }
}

// Run the demo if this file is executed directly
if (require.main === module) {
    const demo = new FHIRPathDemo();
    demo.runAllDemonstrations().catch(error => {
        console.error('❌ Demo failed:', error);
        process.exit(1);
    });
}

module.exports = FHIRPathDemo;
