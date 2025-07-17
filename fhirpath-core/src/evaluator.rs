// FHIRPath Evaluator
//
// This module implements the evaluation of FHIRPath expressions.

use crate::errors::FhirPathError;
use crate::parser::{AstNode, BinaryOperator, UnaryOperator, parse};
use crate::model::{FhirPathValue, FhirResource};
use crate::lexer::tokenize;
use std::collections::HashMap;
use std::io::Read;
use serde::Deserialize;

#[cfg(feature = "trace")]
use log::{debug, trace};

/// Context for FHIRPath evaluation
pub struct EvaluationContext {
    /// The current FHIR resource being evaluated
    pub resource: serde_json::Value,

    /// The current context node in the resource
    pub context: serde_json::Value,

    /// Variables defined in the current scope
    pub variables: HashMap<String, FhirPathValue>,

    /// The current item in a collection during iteration ($this)
    pub this_item: Option<FhirPathValue>,

    /// The current index in a collection during iteration ($index)
    pub index: Option<usize>,

    /// The total number of items in a collection during iteration ($total)
    pub total: Option<usize>,
}

impl EvaluationContext {
    /// Creates a new evaluation context
    pub fn new(resource: serde_json::Value) -> Self {
        Self {
            context: resource.clone(),
            resource,
            variables: HashMap::new(),
            this_item: None,
            index: None,
            total: None,
        }
    }

    /// Sets a variable in the context
    pub fn set_variable(&mut self, name: &str, value: FhirPathValue) {
        self.variables.insert(name.to_string(), value);
    }

    /// Gets a variable from the context
    pub fn get_variable(&self, name: &str) -> Option<&FhirPathValue> {
        self.variables.get(name)
    }

    /// Sets the current item in a collection during iteration ($this)
    pub fn set_this(&mut self, item: FhirPathValue) {
        self.this_item = Some(item);
    }

    /// Gets the current item in a collection during iteration ($this)
    pub fn get_this(&self) -> Option<&FhirPathValue> {
        self.this_item.as_ref()
    }

    /// Sets the current index in a collection during iteration ($index)
    pub fn set_index(&mut self, idx: usize) {
        self.index = Some(idx);
    }

    /// Gets the current index in a collection during iteration ($index)
    pub fn get_index(&self) -> Option<usize> {
        self.index
    }

    /// Sets the total number of items in a collection during iteration ($total)
    pub fn set_total(&mut self, total: usize) {
        self.total = Some(total);
    }

    /// Gets the total number of items in a collection during iteration ($total)
    pub fn get_total(&self) -> Option<usize> {
        self.total
    }

    /// Creates a new context for collection iteration
    pub fn create_iteration_context(&self, item: FhirPathValue, idx: usize, total: usize) -> Result<Self, FhirPathError> {
        let context_value = match &item {
            FhirPathValue::Resource(resource) => serde_json::to_value(resource).map_err(|e| FhirPathError::JsonError(e))?,
            _ => serde_json::to_value(&item).map_err(|e| FhirPathError::JsonError(e))?,
        };

        Ok(Self {
            resource: self.resource.clone(),
            context: context_value,
            variables: self.variables.clone(),
            this_item: Some(item),
            index: Some(idx),
            total: Some(total),
        })
    }
}

/// Trait for visiting AST nodes during evaluation
pub trait AstVisitor {
    /// Called before evaluating an AST node
    fn before_evaluate(&self, node: &AstNode, context: &EvaluationContext);

    /// Called after evaluating an AST node
    fn after_evaluate(&self, node: &AstNode, context: &EvaluationContext, result: &Result<FhirPathValue, FhirPathError>);
}

/// A visitor that logs AST evaluation steps
pub struct LoggingVisitor {
    depth: std::cell::Cell<usize>,
}

impl LoggingVisitor {
    /// Creates a new logging visitor
    pub fn new() -> Self {
        Self {
            depth: std::cell::Cell::new(0),
        }
    }

    fn indent(&self) -> String {
        "  ".repeat(self.depth.get())
    }
}

impl AstVisitor for LoggingVisitor {
    fn before_evaluate(&self, _node: &AstNode, _context: &EvaluationContext) {
        #[cfg(feature = "trace")]
        {
            let indent = self.indent();
            trace!("{}Evaluating: {:?}", indent, _node);
            self.depth.set(self.depth.get() + 1);
        }
    }

    fn after_evaluate(&self, _node: &AstNode, _context: &EvaluationContext, _result: &Result<FhirPathValue, FhirPathError>) {
        #[cfg(feature = "trace")]
        {
            self.depth.set(self.depth.get() - 1);
            let indent = self.indent();
            match _result {
                Ok(value) => trace!("{}Result for {:?}: {:?}", indent, _node, value),
                Err(err) => debug!("{}Error evaluating {:?}: {:?}", indent, _node, err),
            }
        }
    }
}

/// A no-op visitor that does nothing
pub struct NoopVisitor;

impl NoopVisitor {
    /// Creates a new no-op visitor
    pub fn new() -> Self {
        Self
    }
}

impl AstVisitor for NoopVisitor {
    fn before_evaluate(&self, _node: &AstNode, _context: &EvaluationContext) {
        // Do nothing
    }

    fn after_evaluate(&self, _node: &AstNode, _context: &EvaluationContext, _result: &Result<FhirPathValue, FhirPathError>) {
        // Do nothing
    }
}

/// Evaluates a FHIRPath expression AST
pub fn evaluate_ast(node: &AstNode, context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    evaluate_ast_internal(node, context, &NoopVisitor::new())
}

/// Evaluates a FHIRPath expression AST with a custom visitor
pub fn evaluate_ast_with_visitor(node: &AstNode, context: &EvaluationContext, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    visitor.before_evaluate(node, context);
    let result = evaluate_ast_internal(node, context, visitor);
    visitor.after_evaluate(node, context, &result);
    result
}

/// Internal implementation of AST evaluation
fn evaluate_ast_internal(node: &AstNode, context: &EvaluationContext, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    match node {
        AstNode::Identifier(name) => {
            // Check if it's a variable
            if let Some(value) = context.get_variable(name) {
                return Ok(value.clone());
            }

            // Check if we have a FhirResource in this_item and access its properties directly
            if let Some(FhirPathValue::Resource(resource)) = &context.this_item {
                if let Some(value) = resource.properties.get(name) {
                    return json_to_fhirpath_value(value.clone());
                }
            }

            // Check if the identifier matches the resourceType of the root context
            if let serde_json::Value::Object(obj) = &context.context {
                if let Some(serde_json::Value::String(resource_type)) = obj.get("resourceType") {
                    if resource_type == name {
                        // Return the entire resource as a FhirPathValue::Resource
                        return json_to_fhirpath_value(context.context.clone());
                    }
                }

                // Otherwise, try to access the property from the context
                if let Some(value) = obj.get(name) {
                    return json_to_fhirpath_value(value.clone());
                }
            }

            // If not found, return empty
            Ok(FhirPathValue::Empty)
        },

        AstNode::StringLiteral(value) => {
            Ok(FhirPathValue::String(value.clone()))
        },

        AstNode::NumberLiteral(value) => {
            // Determine if it's an integer or decimal
            if value.fract() == 0.0 {
                Ok(FhirPathValue::Integer(*value as i64))
            } else {
                Ok(FhirPathValue::Decimal(*value))
            }
        },

        AstNode::BooleanLiteral(value) => {
            Ok(FhirPathValue::Boolean(*value))
        },

        AstNode::Path(left, right) => {
            // Evaluate the left side
            let left_result = evaluate_ast_with_visitor(left, context, visitor)?;

            // Create a new context with the left result as the context
            match left_result {
                FhirPathValue::Resource(resource) => {
                    let new_context = EvaluationContext {
                        resource: context.resource.clone(),
                        context: serde_json::to_value(&resource).map_err(|e| FhirPathError::JsonError(e))?,
                        variables: context.variables.clone(),
                        this_item: Some(FhirPathValue::Resource(resource)),
                        index: None,
                        total: None,
                    };

                    // Evaluate the right side in the new context
                    evaluate_ast_with_visitor(right, &new_context, visitor)
                },
                FhirPathValue::Collection(items) => {
                    // For collections, evaluate the right side for each item and collect the results
                    let mut results = Vec::new();
                    let total = items.len();

                    for (idx, item) in items.into_iter().enumerate() {
                        match item {
                            FhirPathValue::Resource(resource) => {
                                // Create iteration context with index and total information
                                let new_context = context.create_iteration_context(
                                    FhirPathValue::Resource(resource.clone()),
                                    idx,
                                    total
                                )?;

                                let result = evaluate_ast_with_visitor(right, &new_context, visitor)?;
                                if result != FhirPathValue::Empty {
                                    match result {
                                        FhirPathValue::Collection(mut inner_items) => {
                                            // Flatten collection results
                                            results.append(&mut inner_items);
                                        },
                                        _ => results.push(result),
                                    }
                                }
                            },
                            _ => {
                                // For non-resource items, try to evaluate if they have properties
                                // This allows for handling primitive types with methods
                                let new_context = context.create_iteration_context(item.clone(), idx, total)?;

                                // Only try to evaluate if the right side is an identifier (method call)
                                if let AstNode::Identifier(_) = **right {
                                    let result = evaluate_ast_with_visitor(right, &new_context, visitor)?;
                                    if result != FhirPathValue::Empty {
                                        results.push(result);
                                    }
                                }
                            }
                        }
                    }

                    if results.is_empty() {
                        Ok(FhirPathValue::Empty)
                    } else if results.len() == 1 {
                        // If there's only one result, return it directly
                        Ok(results[0].clone())
                    } else {
                        Ok(FhirPathValue::Collection(results))
                    }
                },
                _ => {
                    // Other types can't have properties
                    Ok(FhirPathValue::Empty)
                }
            }
        },

        AstNode::Indexer { collection, index } => {
            // Evaluate the collection
            let collection_result = evaluate_ast_with_visitor(collection, context, visitor)?;

            // Evaluate the index
            let index_result = evaluate_ast_with_visitor(index, context, visitor)?;

            // Get the item at the specified index
            match (collection_result, index_result) {
                (FhirPathValue::Collection(items), FhirPathValue::Integer(idx)) => {
                    if idx < 0 || idx as usize >= items.len() {
                        Ok(FhirPathValue::Empty)
                    } else {
                        Ok(items[idx as usize].clone())
                    }
                },
                _ => {
                    // Invalid indexing
                    Ok(FhirPathValue::Empty)
                }
            }
        },

        AstNode::FunctionCall { name, arguments } => {
            // Call the appropriate function
            evaluate_function_call(name, arguments, context, visitor)
        },

        AstNode::BinaryOp { op, left, right } => {
            // Evaluate the operands
            let left_result = evaluate_ast_with_visitor(left, context, visitor)?;
            let right_result = evaluate_ast_with_visitor(right, context, visitor)?;

            // Perform the operation
            match op {
                BinaryOperator::Equals => {
                    Ok(FhirPathValue::Boolean(left_result == right_result))
                },
                BinaryOperator::NotEquals => {
                    Ok(FhirPathValue::Boolean(left_result != right_result))
                },
                BinaryOperator::LessThan => {
                    compare_values(&left_result, &right_result, |a, b| a < b)
                },
                BinaryOperator::LessOrEqual => {
                    compare_values(&left_result, &right_result, |a, b| a <= b)
                },
                BinaryOperator::GreaterThan => {
                    compare_values(&left_result, &right_result, |a, b| a > b)
                },
                BinaryOperator::GreaterOrEqual => {
                    compare_values(&left_result, &right_result, |a, b| a >= b)
                },
                BinaryOperator::Addition => {
                    add_values(&left_result, &right_result)
                },
                BinaryOperator::Subtraction => {
                    subtract_values(&left_result, &right_result)
                },
                BinaryOperator::Multiplication => {
                    multiply_values(&left_result, &right_result)
                },
                BinaryOperator::Division => {
                    divide_values(&left_result, &right_result)
                },
                BinaryOperator::And => {
                    match (left_result, right_result) {
                        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                            Ok(FhirPathValue::Boolean(a && b))
                        },
                        _ => {
                            Err(FhirPathError::TypeError("'and' operator requires boolean operands".to_string()))
                        }
                    }
                },
                BinaryOperator::Or => {
                    match (left_result, right_result) {
                        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                            Ok(FhirPathValue::Boolean(a || b))
                        },
                        _ => {
                            Err(FhirPathError::TypeError("'or' operator requires boolean operands".to_string()))
                        }
                    }
                },
                BinaryOperator::Xor => {
                    match (left_result, right_result) {
                        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                            Ok(FhirPathValue::Boolean(a ^ b))
                        },
                        _ => {
                            Err(FhirPathError::TypeError("'xor' operator requires boolean operands".to_string()))
                        }
                    }
                },
                BinaryOperator::Implies => {
                    match (left_result, right_result) {
                        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                            Ok(FhirPathValue::Boolean(!a || b))
                        },
                        _ => {
                            Err(FhirPathError::TypeError("'implies' operator requires boolean operands".to_string()))
                        }
                    }
                },
                BinaryOperator::In => {
                    // TODO: Implement 'in' operator
                    Err(FhirPathError::NotImplemented("'in' operator not yet implemented".to_string()))
                },
            }
        },

        AstNode::UnaryOp { op, operand } => {
            // Evaluate the operand
            let operand_result = evaluate_ast_with_visitor(operand, context, visitor)?;

            // Perform the operation
            match op {
                UnaryOperator::Negate => {
                    match operand_result {
                        FhirPathValue::Integer(value) => {
                            Ok(FhirPathValue::Integer(-value))
                        },
                        FhirPathValue::Decimal(value) => {
                            Ok(FhirPathValue::Decimal(-value))
                        },
                        _ => {
                            Err(FhirPathError::TypeError("Negation requires numeric operand".to_string()))
                        }
                    }
                },
                UnaryOperator::Not => {
                    match operand_result {
                        FhirPathValue::Boolean(value) => {
                            Ok(FhirPathValue::Boolean(!value))
                        },
                        _ => {
                            Err(FhirPathError::TypeError("'not' operator requires boolean operand".to_string()))
                        }
                    }
                },
            }
        },
    }
}

/// Evaluates a FHIRPath expression string
pub fn evaluate_expression(expression: &str, resource: serde_json::Value) -> Result<FhirPathValue, FhirPathError> {
    evaluate_expression_with_visitor(expression, resource, &NoopVisitor::new())
}

/// Evaluates a FHIRPath expression string with a custom visitor
pub fn evaluate_expression_with_visitor(expression: &str, resource: serde_json::Value, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    #[cfg(feature = "trace")]
    debug!("Evaluating FHIRPath expression: {}", expression);

    // Create a context
    let context = EvaluationContext::new(resource);

    // Tokenize and parse the expression
    #[cfg(feature = "trace")]
    trace!("Tokenizing expression");
    let tokens = tokenize(expression)?;

    #[cfg(feature = "trace")]
    trace!("Parsing tokens into AST");
    let ast = parse(&tokens)?;

    #[cfg(feature = "trace")]
    trace!("Starting AST evaluation");

    // Evaluate the AST with the provided visitor
    let result = evaluate_ast_with_visitor(&ast, &context, visitor);

    #[cfg(feature = "trace")]
    match &result {
        Ok(value) => debug!("Expression evaluation result: {:?}", value),
        Err(err) => debug!("Expression evaluation error: {:?}", err),
    }

    result
}

/// Evaluates a FHIRPath expression string using streaming mode for large resources
pub fn evaluate_expression_streaming<R: Read>(expression: &str, reader: R) -> Result<FhirPathValue, FhirPathError> {
    evaluate_expression_streaming_with_visitor(expression, reader, &NoopVisitor::new())
}

/// Evaluates a FHIRPath expression string using streaming mode with a custom visitor
/// This implementation uses streaming JSON parsing to handle large resources efficiently
pub fn evaluate_expression_streaming_with_visitor<R: Read>(expression: &str, mut reader: R, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    #[cfg(feature = "trace")]
    debug!("Evaluating FHIRPath expression with streaming: {}", expression);

    // Tokenize and parse the expression first to understand what we need
    #[cfg(feature = "trace")]
    trace!("Tokenizing expression");
    let tokens = tokenize(expression)?;

    #[cfg(feature = "trace")]
    trace!("Parsing tokens into AST");
    let ast = parse(&tokens)?;

    // For simple expressions that don't require the full resource, we can optimize
    // For now, we still deserialize the full resource but with better memory management
    let mut deserializer = serde_json::Deserializer::from_reader(&mut reader);

    // Use streaming deserialization with memory-conscious approach
    let resource: serde_json::Value = match serde_json::Value::deserialize(&mut deserializer) {
        Ok(value) => value,
        Err(e) => {
            #[cfg(feature = "trace")]
            debug!("JSON deserialization error: {:?}", e);
            return Err(FhirPathError::ParserError(format!("Invalid JSON: {}", e)));
        }
    };

    // Create a context with memory optimization hints
    let context = EvaluationContext::new_with_optimization(resource, true);

    #[cfg(feature = "trace")]
    trace!("Starting AST evaluation with streaming optimizations");

    // Evaluate the AST with the provided visitor
    let result = evaluate_ast_with_visitor(&ast, &context, visitor);

    #[cfg(feature = "trace")]
    match &result {
        Ok(value) => debug!("Expression evaluation result: {:?}", value),
        Err(err) => debug!("Expression evaluation error: {:?}", err),
    }

    result
}

/// Helper function to convert a JSON value to a FHIRPath value
fn json_to_fhirpath_value(value: serde_json::Value) -> Result<FhirPathValue, FhirPathError> {
    match value {
        serde_json::Value::Null => Ok(FhirPathValue::Empty),
        serde_json::Value::Bool(b) => Ok(FhirPathValue::Boolean(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(FhirPathValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(FhirPathValue::Decimal(f))
            } else {
                Err(FhirPathError::TypeError("Invalid number".to_string()))
            }
        },
        serde_json::Value::String(s) => Ok(FhirPathValue::String(s)),
        serde_json::Value::Array(arr) => {
            let mut items = Vec::new();
            for item in arr {
                items.push(json_to_fhirpath_value(item)?);
            }
            Ok(FhirPathValue::Collection(items))
        },
        serde_json::Value::Object(obj) => {
            // Check if it's a FHIR resource
            if obj.contains_key("resourceType") {
                let resource = FhirResource::from_json(serde_json::Value::Object(obj))?;
                Ok(FhirPathValue::Resource(resource))
            } else {
                // Convert to a resource without a resourceType
                let resource = FhirResource {
                    resource_type: None,
                    properties: obj.into_iter().collect(),
                };
                Ok(FhirPathValue::Resource(resource))
            }
        },
    }
}

/// Helper function for comparison operations
fn compare_values<F>(left: &FhirPathValue, right: &FhirPathValue, compare_fn: F) -> Result<FhirPathValue, FhirPathError>
where
    F: Fn(f64, f64) -> bool,
{
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a as f64, *b as f64)))
        },
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a as f64, *b)))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a, *b as f64)))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a, *b)))
        },
        (FhirPathValue::String(a), FhirPathValue::String(b)) => {
            // String comparison
            Ok(FhirPathValue::Boolean(compare_fn(a.cmp(b) as i32 as f64, 0.0)))
        },
        _ => {
            Err(FhirPathError::TypeError("Comparison requires compatible operands".to_string()))
        }
    }
}

/// Helper function for addition
fn add_values(left: &FhirPathValue, right: &FhirPathValue) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Integer(a + b))
        },
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 + b))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a + *b as f64))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(a + b))
        },
        (FhirPathValue::String(a), FhirPathValue::String(b)) => {
            // String concatenation
            Ok(FhirPathValue::String(format!("{}{}", a, b)))
        },
        (FhirPathValue::Collection(a), FhirPathValue::Collection(b)) => {
            // Collection union
            let mut result = a.clone();
            result.extend(b.clone());
            Ok(FhirPathValue::Collection(result))
        },
        _ => {
            Err(FhirPathError::TypeError("Addition requires compatible operands".to_string()))
        }
    }
}

/// Helper function for subtraction
fn subtract_values(left: &FhirPathValue, right: &FhirPathValue) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Integer(a - b))
        },
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 - b))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a - *b as f64))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(a - b))
        },
        _ => {
            Err(FhirPathError::TypeError("Subtraction requires numeric operands".to_string()))
        }
    }
}

/// Helper function for multiplication
fn multiply_values(left: &FhirPathValue, right: &FhirPathValue) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Integer(a * b))
        },
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 * b))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a * *b as f64))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(a * b))
        },
        _ => {
            Err(FhirPathError::TypeError("Multiplication requires numeric operands".to_string()))
        }
    }
}

/// Helper function for division
fn divide_values(left: &FhirPathValue, right: &FhirPathValue) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (_, FhirPathValue::Integer(b)) if *b == 0 => {
            Err(FhirPathError::EvaluationError("Division by zero".to_string()))
        },
        (_, FhirPathValue::Decimal(b)) if *b == 0.0 => {
            Err(FhirPathError::EvaluationError("Division by zero".to_string()))
        },
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            // Integer division results in a decimal
            Ok(FhirPathValue::Decimal(*a as f64 / *b as f64))
        },
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 / b))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a / *b as f64))
        },
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(a / b))
        },
        _ => {
            Err(FhirPathError::TypeError("Division requires numeric operands".to_string()))
        }
    }
}

/// Evaluates a function call with proper argument handling
fn evaluate_function_call(name: &str, arguments: &[AstNode], context: &EvaluationContext, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    match name {
        // Collection filtering and projection functions
        "where" => evaluate_where_function(arguments, context, visitor),
        "select" => evaluate_select_function(arguments, context, visitor),

        // Collection navigation functions
        "first" => evaluate_first_function(arguments, context),
        "last" => evaluate_last_function(arguments, context),
        "tail" => evaluate_tail_function(arguments, context),
        "skip" => evaluate_skip_function(arguments, context, visitor),
        "take" => evaluate_take_function(arguments, context),

        // Collection testing functions
        "exists" => evaluate_exists_function(arguments, context),
        "empty" => evaluate_empty_function(arguments, context),
        "count" => evaluate_count_function(arguments, context),
        "length" => evaluate_length_function(arguments, context),

        // Collection aggregation functions
        "distinct" => evaluate_distinct_function(arguments, context),
        "union" => evaluate_union_function(arguments, context),
        "intersect" => evaluate_intersect_function(arguments, context),

        // Type checking functions
        "is" => evaluate_is_function(arguments, context),
        "as" => evaluate_as_function(arguments, context),

        // String functions
        "contains" => evaluate_contains_function(arguments, context),
        "startsWith" => evaluate_starts_with_function(arguments, context),
        "endsWith" => evaluate_ends_with_function(arguments, context),
        "substring" => evaluate_substring_function(arguments, context),
        "indexOf" => evaluate_index_of_function(arguments, context),
        "replace" => evaluate_replace_function(arguments, context),
        "matches" => evaluate_matches_function(arguments, context),
        "split" => evaluate_split_function(arguments, context),

        // Math functions
        "abs" => evaluate_abs_function(arguments, context),
        "ceiling" => evaluate_ceiling_function(arguments, context),
        "floor" => evaluate_floor_function(arguments, context),
        "round" => evaluate_round_function(arguments, context),
        "sqrt" => evaluate_sqrt_function(arguments, context),
        "exp" => evaluate_exp_function(arguments, context),
        "ln" => evaluate_ln_function(arguments, context),
        "log" => evaluate_log_function(arguments, context),
        "power" => evaluate_power_function(arguments, context),

        // Date/time functions
        "now" => evaluate_now_function(arguments, context),
        "today" => evaluate_today_function(arguments, context),
        "timeOfDay" => evaluate_time_of_day_function(arguments, context),

        _ => Err(FhirPathError::EvaluationError(format!("Unknown function: {}", name)))
    }
}

/// Evaluates the where() function for filtering collections
fn evaluate_where_function(arguments: &[AstNode], context: &EvaluationContext, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!("'where' function expects 1 argument, got {}", arguments.len())));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;
    let total = collection.len();

    // For memory efficiency on large collections, process in chunks
    const CHUNK_SIZE: usize = 1000;
    let mut results = Vec::new();

    if total > CHUNK_SIZE {
        // Process large collections in chunks to reduce memory usage
        for chunk_start in (0..total).step_by(CHUNK_SIZE) {
            let chunk_end = std::cmp::min(chunk_start + CHUNK_SIZE, total);
            let chunk = &collection[chunk_start..chunk_end];

            for (relative_idx, item) in chunk.iter().enumerate() {
                let idx = chunk_start + relative_idx;
                // Create a new context for this item
                let item_context = context.create_iteration_context(item.clone(), idx, total)?;

                // Evaluate the filter expression in this context
                let filter_result = evaluate_ast_with_visitor(&arguments[0], &item_context, visitor)?;

                // Check if the filter evaluates to true
                if is_truthy(&filter_result) {
                    results.push(item.clone());
                }
            }
        }
    } else {
        // For smaller collections, use the original approach
        for (idx, item) in collection.into_iter().enumerate() {
            // Create a new context for this item
            let item_context = context.create_iteration_context(item.clone(), idx, total)?;

            // Evaluate the filter expression in this context
            let filter_result = evaluate_ast_with_visitor(&arguments[0], &item_context, visitor)?;

            // Check if the filter evaluates to true
            if is_truthy(&filter_result) {
                results.push(item);
            }
        }
    }

    if results.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(results))
    }
}

/// Evaluates the select() function for projection
fn evaluate_select_function(arguments: &[AstNode], context: &EvaluationContext, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!("'select' function expects 1 argument, got {}", arguments.len())));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;
    let mut results = Vec::new();
    let total = collection.len();

    // Apply the projection to each item
    for (idx, item) in collection.into_iter().enumerate() {
        // Create a new context for this item
        let item_context = context.create_iteration_context(item, idx, total)?;

        // Evaluate the projection expression in this context
        let projection_result = evaluate_ast_with_visitor(&arguments[0], &item_context, visitor)?;

        // Add the result to the collection
        if projection_result != FhirPathValue::Empty {
            match projection_result {
                FhirPathValue::Collection(mut inner_items) => {
                    results.append(&mut inner_items);
                },
                _ => results.push(projection_result),
            }
        }
    }

    if results.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(results))
    }
}

/// Evaluates the first() function
fn evaluate_first_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'first' function expects 0 arguments, got {}", arguments.len())));
    }

    let collection = get_current_collection(context)?;
    if collection.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(collection[0].clone())
    }
}

/// Evaluates the last() function
fn evaluate_last_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'last' function expects 0 arguments, got {}", arguments.len())));
    }

    let collection = get_current_collection(context)?;
    if collection.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(collection[collection.len() - 1].clone())
    }
}

/// Evaluates the tail() function
fn evaluate_tail_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'tail' function expects 0 arguments, got {}", arguments.len())));
    }

    let collection = get_current_collection(context)?;
    if collection.len() <= 1 {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(collection[1..].to_vec()))
    }
}

/// Evaluates the skip() function
fn evaluate_skip_function(arguments: &[AstNode], context: &EvaluationContext, visitor: &dyn AstVisitor) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!("'skip' function expects 1 argument, got {}", arguments.len())));
    }

    let skip_count_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
    let skip_count = match skip_count_result {
        FhirPathValue::Integer(n) => n as usize,
        _ => return Err(FhirPathError::TypeError("'skip' function requires an integer argument".to_string())),
    };

    let collection = get_current_collection(context)?;
    if skip_count >= collection.len() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(collection[skip_count..].to_vec()))
    }
}

/// Evaluates the take() function
fn evaluate_take_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!("'take' function expects 1 argument, got {}", arguments.len())));
    }

    let take_count_result = evaluate_ast(&arguments[0], context)?;
    let take_count = match take_count_result {
        FhirPathValue::Integer(n) => n as usize,
        _ => return Err(FhirPathError::TypeError("'take' function requires an integer argument".to_string())),
    };

    let collection = get_current_collection(context)?;
    let end_index = std::cmp::min(take_count, collection.len());

    if end_index == 0 {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(collection[..end_index].to_vec()))
    }
}

/// Evaluates the exists() function
fn evaluate_exists_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() > 1 {
        return Err(FhirPathError::EvaluationError(format!("'exists' function expects 0 or 1 arguments, got {}", arguments.len())));
    }

    if arguments.is_empty() {
        // Check if the current context has any values
        match get_current_collection(context) {
            Ok(collection) => Ok(FhirPathValue::Boolean(!collection.is_empty())),
            Err(_) => Ok(FhirPathValue::Boolean(false)),
        }
    } else {
        // Check if any item in the collection satisfies the condition
        let collection = get_current_collection(context)?;
        let total = collection.len();

        for (idx, item) in collection.into_iter().enumerate() {
            let item_context = context.create_iteration_context(item, idx, total)?;
            let condition_result = evaluate_ast(&arguments[0], &item_context)?;

            if is_truthy(&condition_result) {
                return Ok(FhirPathValue::Boolean(true));
            }
        }

        Ok(FhirPathValue::Boolean(false))
    }
}

/// Evaluates the empty() function
fn evaluate_empty_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'empty' function expects 0 arguments, got {}", arguments.len())));
    }

    match get_current_collection(context) {
        Ok(collection) => Ok(FhirPathValue::Boolean(collection.is_empty())),
        Err(_) => Ok(FhirPathValue::Boolean(true)),
    }
}

/// Evaluates the count() function
fn evaluate_count_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'count' function expects 0 arguments, got {}", arguments.len())));
    }

    let collection = get_current_collection(context)?;
    Ok(FhirPathValue::Integer(collection.len() as i64))
}

/// Evaluates the length() function for strings
fn evaluate_length_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'length' function expects 0 arguments, got {}", arguments.len())));
    }

    // Get the current value from context
    match &context.context {
        serde_json::Value::String(s) => Ok(FhirPathValue::Integer(s.len() as i64)),
        _ => Err(FhirPathError::TypeError("'length' function can only be applied to strings".to_string())),
    }
}

/// Evaluates the distinct() function
fn evaluate_distinct_function(arguments: &[AstNode], context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!("'distinct' function expects 0 arguments, got {}", arguments.len())));
    }

    let collection = get_current_collection(context)?;
    let mut unique_items = Vec::new();

    for item in collection {
        if !unique_items.iter().any(|existing| values_equal(existing, &item)) {
            unique_items.push(item);
        }
    }

    if unique_items.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(unique_items))
    }
}

/// Placeholder implementations for remaining functions
fn evaluate_union_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'union' function not yet implemented".to_string()))
}

fn evaluate_intersect_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'intersect' function not yet implemented".to_string()))
}

fn evaluate_is_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'is' function not yet implemented".to_string()))
}

fn evaluate_as_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'as' function not yet implemented".to_string()))
}

fn evaluate_contains_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'contains' function not yet implemented".to_string()))
}

fn evaluate_starts_with_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'startsWith' function not yet implemented".to_string()))
}

fn evaluate_ends_with_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'endsWith' function not yet implemented".to_string()))
}

fn evaluate_substring_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'substring' function not yet implemented".to_string()))
}

fn evaluate_index_of_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'indexOf' function not yet implemented".to_string()))
}

fn evaluate_replace_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'replace' function not yet implemented".to_string()))
}

fn evaluate_matches_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'matches' function not yet implemented".to_string()))
}

fn evaluate_split_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'split' function not yet implemented".to_string()))
}

fn evaluate_abs_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'abs' function not yet implemented".to_string()))
}

fn evaluate_ceiling_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'ceiling' function not yet implemented".to_string()))
}

fn evaluate_floor_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'floor' function not yet implemented".to_string()))
}

fn evaluate_round_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'round' function not yet implemented".to_string()))
}

fn evaluate_sqrt_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'sqrt' function not yet implemented".to_string()))
}

fn evaluate_exp_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'exp' function not yet implemented".to_string()))
}

fn evaluate_ln_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'ln' function not yet implemented".to_string()))
}

fn evaluate_log_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'log' function not yet implemented".to_string()))
}

fn evaluate_power_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'power' function not yet implemented".to_string()))
}

fn evaluate_now_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'now' function not yet implemented".to_string()))
}

fn evaluate_today_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'today' function not yet implemented".to_string()))
}

fn evaluate_time_of_day_function(_arguments: &[AstNode], _context: &EvaluationContext) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented("'timeOfDay' function not yet implemented".to_string()))
}

/// Helper function to get the current collection from context
fn get_current_collection(context: &EvaluationContext) -> Result<Vec<FhirPathValue>, FhirPathError> {
    match &context.this_item {
        Some(FhirPathValue::Collection(items)) => Ok(items.clone()),
        Some(item) => Ok(vec![item.clone()]),
        None => {
            // Try to get from the main context
            match &context.context {
                serde_json::Value::Array(arr) => {
                    let mut items = Vec::new();
                    for value in arr {
                        items.push(json_to_fhirpath_value(value.clone())?);
                    }
                    Ok(items)
                },
                value => Ok(vec![json_to_fhirpath_value(value.clone())?]),
            }
        }
    }
}

/// Helper function to check if a value is truthy
fn is_truthy(value: &FhirPathValue) -> bool {
    match value {
        FhirPathValue::Empty => false,
        FhirPathValue::Boolean(b) => *b,
        FhirPathValue::Integer(i) => *i != 0,
        FhirPathValue::Decimal(d) => *d != 0.0,
        FhirPathValue::String(s) => !s.is_empty(),
        FhirPathValue::Collection(items) => !items.is_empty(),
        _ => true,
    }
}

/// Helper function to check if two values are equal
fn values_equal(left: &FhirPathValue, right: &FhirPathValue) -> bool {
    match (left, right) {
        (FhirPathValue::Empty, FhirPathValue::Empty) => true,
        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => a == b,
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => a == b,
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => (a - b).abs() < f64::EPSILON,
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => (*a as f64 - b).abs() < f64::EPSILON,
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => (a - *b as f64).abs() < f64::EPSILON,
        (FhirPathValue::String(a), FhirPathValue::String(b)) => a == b,
        (FhirPathValue::Date(a), FhirPathValue::Date(b)) => a == b,
        (FhirPathValue::DateTime(a), FhirPathValue::DateTime(b)) => a == b,
        (FhirPathValue::Time(a), FhirPathValue::Time(b)) => a == b,
        (FhirPathValue::Quantity { value: v1, unit: u1 }, FhirPathValue::Quantity { value: v2, unit: u2 }) => {
            (v1 - v2).abs() < f64::EPSILON && u1 == u2
        },
        _ => false,
    }
}
