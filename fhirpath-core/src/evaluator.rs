// FHIRPath Evaluator
//
// This module implements the evaluation of FHIRPath expressions.

use crate::errors::FhirPathError;
use crate::lexer::tokenize;
use crate::model::{FhirPathValue, FhirResource};
use crate::parser::{parse, AstNode, BinaryOperator, UnaryOperator};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Read;

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

    /// Optimization settings
    pub optimization_enabled: bool,

    /// Cache for expression results
    pub expression_cache: HashMap<u64, FhirPathValue>,
}

impl EvaluationContext {
    /// Initialize standard FHIRPath variables
    fn init_standard_variables() -> HashMap<String, FhirPathValue> {
        let mut variables = HashMap::new();

        // Standard FHIRPath variables
        variables.insert("sct".to_string(), FhirPathValue::String("http://snomed.info/sct".to_string()));
        variables.insert("loinc".to_string(), FhirPathValue::String("http://loinc.org".to_string()));
        variables.insert("ucum".to_string(), FhirPathValue::String("http://unitsofmeasure.org".to_string()));

        variables
    }

    /// Creates a new evaluation context
    pub fn new(resource: serde_json::Value) -> Self {
        Self {
            context: resource.clone(),
            resource,
            variables: Self::init_standard_variables(),
            this_item: None,
            index: None,
            total: None,
            optimization_enabled: false,
            expression_cache: HashMap::new(),
        }
    }

    /// Creates a new evaluation context with optimization settings
    pub fn new_with_optimization(resource: serde_json::Value, optimization_enabled: bool) -> Self {
        Self {
            context: resource.clone(),
            resource,
            variables: Self::init_standard_variables(),
            this_item: None,
            index: None,
            total: None,
            optimization_enabled,
            expression_cache: HashMap::new(),
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
    pub fn create_iteration_context(
        &self,
        item: FhirPathValue,
        idx: usize,
        total: usize,
    ) -> Result<Self, FhirPathError> {
        let context_value = match &item {
            FhirPathValue::Resource(resource) => {
                serde_json::to_value(resource).map_err(FhirPathError::JsonError)?
            }
            _ => serde_json::to_value(&item).map_err(FhirPathError::JsonError)?,
        };

        Ok(Self {
            resource: self.resource.clone(),
            context: context_value,
            variables: self.variables.clone(),
            this_item: Some(item),
            index: Some(idx),
            total: Some(total),
            optimization_enabled: self.optimization_enabled,
            expression_cache: HashMap::new(),
        })
    }
}

/// Trait for visiting AST nodes during evaluation
pub trait AstVisitor {
    /// Called before evaluating an AST node
    fn before_evaluate(&self, node: &AstNode, context: &EvaluationContext);

    /// Called after evaluating an AST node
    fn after_evaluate(
        &self,
        node: &AstNode,
        context: &EvaluationContext,
        result: &Result<FhirPathValue, FhirPathError>,
    );
}

/// A visitor that logs AST evaluation steps
pub struct LoggingVisitor {
    depth: std::cell::Cell<usize>,
}

impl Default for LoggingVisitor {
    fn default() -> Self {
        Self::new()
    }
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

    fn after_evaluate(
        &self,
        _node: &AstNode,
        _context: &EvaluationContext,
        _result: &Result<FhirPathValue, FhirPathError>,
    ) {
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

impl Default for NoopVisitor {
    fn default() -> Self {
        Self::new()
    }
}

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

    fn after_evaluate(
        &self,
        _node: &AstNode,
        _context: &EvaluationContext,
        _result: &Result<FhirPathValue, FhirPathError>,
    ) {
        // Do nothing
    }
}

/// Evaluates a FHIRPath expression AST
pub fn evaluate_ast(
    node: &AstNode,
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    evaluate_ast_internal(node, context, &NoopVisitor::new())
}

/// Evaluates a FHIRPath expression AST with a custom visitor
pub fn evaluate_ast_with_visitor(
    node: &AstNode,
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    visitor.before_evaluate(node, context);
    let result = evaluate_ast_internal(node, context, visitor);
    visitor.after_evaluate(node, context, &result);
    result
}

/// Evaluates a FHIRPath expression AST with a mutable context for caching
pub fn evaluate_ast_with_caching(
    node: &AstNode,
    context: &mut EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    visitor.before_evaluate(node, context);

    // Check cache if optimization is enabled and the node is worth caching
    if context.optimization_enabled && should_cache_node(node) {
        let cache_key = generate_cache_key(node);
        if let Some(cached_result) = context.expression_cache.get(&cache_key) {
            let result = Ok(cached_result.clone());
            visitor.after_evaluate(node, context, &result);
            return result;
        }
    }

    let result = evaluate_ast_internal_uncached(node, context, visitor);

    // Cache the result if optimization is enabled, evaluation was successful, and the node is worth caching
    if context.optimization_enabled && should_cache_node(node) {
        if let Ok(ref value) = result {
            let cache_key = generate_cache_key(node);
            // Limit cache size to prevent memory bloat
            if context.expression_cache.len() < 1000 {
                context.expression_cache.insert(cache_key, value.clone());
            }
        }
    }

    visitor.after_evaluate(node, context, &result);
    result
}

/// Internal implementation of AST evaluation
fn evaluate_ast_internal(
    node: &AstNode,
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    evaluate_ast_internal_uncached(node, context, visitor)
}

/// Internal implementation of AST evaluation without caching
fn evaluate_ast_internal_uncached(
    node: &AstNode,
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
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
        }

        AstNode::StringLiteral(value) => Ok(FhirPathValue::String(value.clone())),

        AstNode::NumberLiteral(value) => {
            // Determine if it's an integer or decimal
            if value.fract() == 0.0 {
                Ok(FhirPathValue::Integer(*value as i64))
            } else {
                Ok(FhirPathValue::Decimal(*value))
            }
        }

        AstNode::BooleanLiteral(value) => Ok(FhirPathValue::Boolean(*value)),

        AstNode::DateTimeLiteral(value) => {
            // Parse the datetime literal (starts with @)
            let datetime_str = if value.starts_with('@') {
                &value[1..] // Remove the @ prefix
            } else {
                value
            };

            // Determine if this is a Date, DateTime, or Time
            if datetime_str.starts_with('T') {
                // Starts with 'T', so it's a Time literal (e.g., T14:34:28)
                Ok(FhirPathValue::Time(datetime_str.to_string()))
            } else if datetime_str.contains('T') || datetime_str.ends_with('T') {
                // Contains 'T' or ends with 'T' (like "2015T"), so it's a DateTime
                Ok(FhirPathValue::DateTime(datetime_str.to_string()))
            } else {
                // No 'T', so it's a Date
                Ok(FhirPathValue::Date(datetime_str.to_string()))
            }
        }

        AstNode::Variable(name) => {
            // Look up variable in the evaluation context
            if let Some(value) = context.get_variable(name) {
                Ok(value.clone())
            } else {
                // Variable not found, return empty
                Ok(FhirPathValue::Empty)
            }
        }

        AstNode::Path(left, right) => {
            // Evaluate the left side
            let left_result = evaluate_ast_with_visitor(left, context, visitor)?;
            // Create a new context with the left result as the context
            match left_result {
                FhirPathValue::Resource(resource) => {
                    let new_context = EvaluationContext {
                        resource: context.resource.clone(),
                        context: serde_json::to_value(&resource)
                            .map_err(FhirPathError::JsonError)?,
                        variables: context.variables.clone(),
                        this_item: Some(FhirPathValue::Resource(resource)),
                        index: None,
                        total: None,
                        optimization_enabled: context.optimization_enabled,
                        expression_cache: HashMap::new(),
                    };

                    // Evaluate the right side in the new context
                    evaluate_ast_with_visitor(right, &new_context, visitor)
                }
                FhirPathValue::Collection(items) => {
                    // Check if the right side is a function call - if so, call it on the entire collection
                    match **right {
                        AstNode::FunctionCall { .. } => {
                            // Create a new context with the collection as this_item for function calls
                            let new_context = EvaluationContext {
                                resource: context.resource.clone(),
                                context: context.context.clone(),
                                variables: context.variables.clone(),
                                this_item: Some(FhirPathValue::Collection(items)),
                                index: None,
                                total: None,
                                optimization_enabled: context.optimization_enabled,
                                expression_cache: HashMap::new(),
                            };

                            // Evaluate the function call in the new context
                            evaluate_ast_with_visitor(right, &new_context, visitor)
                        }
                        _ => {
                            // For non-function calls, evaluate the right side for each item and collect the results
                            let mut results = Vec::new();
                            let total = items.len();

                            for (idx, item) in items.into_iter().enumerate() {
                                match item {
                                    FhirPathValue::Resource(resource) => {
                                        // Create an iteration context with index and total information
                                        let new_context = context.create_iteration_context(
                                            FhirPathValue::Resource(resource.clone()),
                                            idx,
                                            total,
                                        )?;

                                        let result =
                                            evaluate_ast_with_visitor(right, &new_context, visitor)?;
                                        if result != FhirPathValue::Empty {
                                            match result {
                                                FhirPathValue::Collection(mut inner_items) => {
                                                    // Flatten collection results
                                                    results.append(&mut inner_items);
                                                }
                                                _ => results.push(result),
                                            }
                                        }
                                    }
                                    _ => {
                                        // For non-resource items, try to evaluate if they have properties
                                        // This allows for handling primitive types with methods
                                        let new_context =
                                            context.create_iteration_context(item.clone(), idx, total)?;

                                        // Only try to evaluate if the right side is an identifier (method call)
                                        if let AstNode::Identifier(_) = **right {
                                            let result =
                                                evaluate_ast_with_visitor(right, &new_context, visitor)?;
                                            if result != FhirPathValue::Empty {
                                                results.push(result);
                                            }
                                        }
                                    }
                                }
                            }

                            if results.is_empty() {
                                // For property access on empty collections, return empty
                                Ok(FhirPathValue::Empty)
                            } else if results.len() == 1 {
                                // If there's only one result, return it directly
                                Ok(results[0].clone())
                            } else {
                                Ok(FhirPathValue::Collection(results))
                            }
                        }
                    }
                }
                FhirPathValue::Empty => {
                    // For empty results, check if the right side is a function call
                    match **right {
                        AstNode::FunctionCall { .. } => {
                            // Create a new context with the left result as this_item for function calls
                            let new_context = EvaluationContext {
                                resource: context.resource.clone(),
                                context: context.context.clone(),
                                variables: context.variables.clone(),
                                this_item: Some(left_result),
                                index: None,
                                total: None,
                                optimization_enabled: context.optimization_enabled,
                                expression_cache: HashMap::new(),
                            };

                            // Evaluate the function call in the new context
                            evaluate_ast_with_visitor(right, &new_context, visitor)
                        }
                        _ => {
                            // Empty results can't have properties (only function calls are allowed)
                            Ok(FhirPathValue::Empty)
                        }
                    }
                }
                _ => {
                    // For primitive types (String, Integer, etc.), check if the right side is a function call
                    match **right {
                        AstNode::FunctionCall { .. } => {
                            // Create a new context with the left result as this_item for function calls
                            let new_context = EvaluationContext {
                                resource: context.resource.clone(),
                                context: context.context.clone(),
                                variables: context.variables.clone(),
                                this_item: Some(left_result),
                                index: None,
                                total: None,
                                optimization_enabled: context.optimization_enabled,
                                expression_cache: HashMap::new(),
                            };

                            // Evaluate the function call in the new context
                            evaluate_ast_with_visitor(right, &new_context, visitor)
                        }
                        _ => {
                            // Other types can't have properties (only function calls are allowed)
                            Ok(FhirPathValue::Empty)
                        }
                    }
                }
            }
        }

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
                }
                _ => {
                    // Invalid indexing
                    Ok(FhirPathValue::Empty)
                }
            }
        }

        AstNode::FunctionCall { name, arguments } => {
            // Call the appropriate function
            evaluate_function_call(name, arguments, context, visitor)
        }

        AstNode::BinaryOp { op, left, right } => {
            // Evaluate the operands
            let left_result = evaluate_ast_with_visitor(left, context, visitor)?;
            let right_result = evaluate_ast_with_visitor(right, context, visitor)?;

            // Perform the operation
            match op {
                BinaryOperator::Equals => Ok(FhirPathValue::Boolean(values_equal(&left_result, &right_result))),
                BinaryOperator::NotEquals => {
                    Ok(FhirPathValue::Boolean(!values_equal(&left_result, &right_result)))
                }
                BinaryOperator::LessThan => {
                    compare_values(&left_result, &right_result, |a, b| a < b)
                }
                BinaryOperator::LessOrEqual => {
                    compare_values(&left_result, &right_result, |a, b| a <= b)
                }
                BinaryOperator::GreaterThan => {
                    compare_values(&left_result, &right_result, |a, b| a > b)
                }
                BinaryOperator::GreaterOrEqual => {
                    compare_values(&left_result, &right_result, |a, b| a >= b)
                }
                BinaryOperator::Addition => add_values(&left_result, &right_result),
                BinaryOperator::Subtraction => subtract_values(&left_result, &right_result),
                BinaryOperator::Multiplication => multiply_values(&left_result, &right_result),
                BinaryOperator::Division => divide_values(&left_result, &right_result),
                BinaryOperator::Mod => mod_values(&left_result, &right_result),
                BinaryOperator::And => match (left_result, right_result) {
                    (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                        Ok(FhirPathValue::Boolean(a && b))
                    }
                    _ => Err(FhirPathError::TypeError(
                        "'and' operator requires boolean operands".to_string(),
                    )),
                },
                BinaryOperator::Or => match (left_result, right_result) {
                    (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                        Ok(FhirPathValue::Boolean(a || b))
                    }
                    _ => Err(FhirPathError::TypeError(
                        "'or' operator requires boolean operands".to_string(),
                    )),
                },
                BinaryOperator::Xor => match (left_result, right_result) {
                    (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                        Ok(FhirPathValue::Boolean(a ^ b))
                    }
                    _ => Err(FhirPathError::TypeError(
                        "'xor' operator requires boolean operands".to_string(),
                    )),
                },
                BinaryOperator::Implies => match (left_result, right_result) {
                    (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
                        Ok(FhirPathValue::Boolean(!a || b))
                    }
                    _ => Err(FhirPathError::TypeError(
                        "'implies' operator requires boolean operands".to_string(),
                    )),
                },
                BinaryOperator::In => {
                    // 'in' operator checks if left operand is contained in right operand collection
                    match right_result {
                        FhirPathValue::Collection(items) => {
                            let found = items.iter().any(|item| values_equal(&left_result, item));
                            Ok(FhirPathValue::Boolean(found))
                        }
                        FhirPathValue::Empty => Ok(FhirPathValue::Boolean(false)),
                        other => {
                            // Single item on right side
                            Ok(FhirPathValue::Boolean(values_equal(&left_result, &other)))
                        }
                    }
                }
                BinaryOperator::Union => {
                    // Union operator combines two collections, removing duplicates
                    let mut result_items = Vec::new();

                    // Add items from left operand
                    match left_result {
                        FhirPathValue::Collection(items) => {
                            result_items.extend(items);
                        }
                        FhirPathValue::Empty => {
                            // Empty contributes nothing
                        }
                        other => {
                            result_items.push(other);
                        }
                    }

                    // Add items from right operand
                    match right_result {
                        FhirPathValue::Collection(items) => {
                            for item in items {
                                // Only add if not already present (remove duplicates)
                                if !result_items
                                    .iter()
                                    .any(|existing| values_equal(existing, &item))
                                {
                                    result_items.push(item);
                                }
                            }
                        }
                        FhirPathValue::Empty => {
                            // Empty contributes nothing
                        }
                        other => {
                            // Only add if not already present
                            if !result_items
                                .iter()
                                .any(|existing| values_equal(existing, &other))
                            {
                                result_items.push(other);
                            }
                        }
                    }

                    if result_items.is_empty() {
                        Ok(FhirPathValue::Empty)
                    } else {
                        Ok(FhirPathValue::Collection(result_items))
                    }
                }
                BinaryOperator::Concatenation => {
                    // Concatenation operator (&) converts operands to strings and concatenates them
                    let left_str = match left_result {
                        FhirPathValue::String(s) => s,
                        FhirPathValue::Integer(i) => i.to_string(),
                        FhirPathValue::Decimal(d) => d.to_string(),
                        FhirPathValue::Boolean(b) => b.to_string(),
                        FhirPathValue::Empty => String::new(),
                        FhirPathValue::Collection(ref items) if items.is_empty() => String::new(),
                        _ => return Err(FhirPathError::TypeError(
                            "Cannot convert left operand to string for concatenation".to_string(),
                        )),
                    };

                    let right_str = match right_result {
                        FhirPathValue::String(s) => s,
                        FhirPathValue::Integer(i) => i.to_string(),
                        FhirPathValue::Decimal(d) => d.to_string(),
                        FhirPathValue::Boolean(b) => b.to_string(),
                        FhirPathValue::Empty => String::new(),
                        FhirPathValue::Collection(ref items) if items.is_empty() => String::new(),
                        _ => return Err(FhirPathError::TypeError(
                            "Cannot convert right operand to string for concatenation".to_string(),
                        )),
                    };

                    Ok(FhirPathValue::String(format!("{}{}", left_str, right_str)))
                }
            }
        }

        AstNode::UnaryOp { op, operand } => {
            // Evaluate the operand
            let operand_result = evaluate_ast_with_visitor(operand, context, visitor)?;

            // Perform the operation
            match op {
                UnaryOperator::Negate => match operand_result {
                    FhirPathValue::Integer(value) => Ok(FhirPathValue::Integer(-value)),
                    FhirPathValue::Decimal(value) => Ok(FhirPathValue::Decimal(-value)),
                    _ => Err(FhirPathError::TypeError(
                        "Negation requires numeric operand".to_string(),
                    )),
                },
                UnaryOperator::Not => match operand_result {
                    FhirPathValue::Boolean(b) => Ok(FhirPathValue::Boolean(!b)),
                    FhirPathValue::Empty => Ok(FhirPathValue::Boolean(true)),
                    FhirPathValue::Collection(ref items) if items.is_empty() => {
                        Ok(FhirPathValue::Boolean(true))
                    }
                    _ => Ok(FhirPathValue::Boolean(false)),
                },
            }
        }
    }
}

/// Evaluates a FHIRPath expression string
pub fn evaluate_expression(
    expression: &str,
    resource: serde_json::Value,
) -> Result<FhirPathValue, FhirPathError> {
    evaluate_expression_with_visitor(expression, resource, &NoopVisitor::new())
}

/// Evaluates a FHIRPath expression string with optimization enabled
pub fn evaluate_expression_optimized(
    expression: &str,
    resource: serde_json::Value,
) -> Result<FhirPathValue, FhirPathError> {
    let tokens = tokenize(expression)?;
    let ast = parse(&tokens)?;
    let optimized_ast = optimize_ast(&ast);
    let mut context = EvaluationContext::new_with_optimization(resource, true);
    let visitor = NoopVisitor::new();
    evaluate_ast_with_caching(&optimized_ast, &mut context, &visitor)
}

/// Optimizes an AST by applying various optimization techniques
fn optimize_ast(node: &AstNode) -> AstNode {
    match node {
        // Constant folding for binary operations
        AstNode::BinaryOp { op, left, right } => {
            let optimized_left = optimize_ast(left);
            let optimized_right = optimize_ast(right);

            // Try to fold constants
            match (&optimized_left, &optimized_right) {
                (AstNode::BooleanLiteral(left_val), AstNode::BooleanLiteral(right_val)) => match op
                {
                    BinaryOperator::And => AstNode::BooleanLiteral(*left_val && *right_val),
                    BinaryOperator::Or => AstNode::BooleanLiteral(*left_val || *right_val),
                    BinaryOperator::Equals => AstNode::BooleanLiteral(*left_val == *right_val),
                    BinaryOperator::NotEquals => AstNode::BooleanLiteral(*left_val != *right_val),
                    _ => AstNode::BinaryOp {
                        op: op.clone(),
                        left: Box::new(optimized_left),
                        right: Box::new(optimized_right),
                    },
                },
                (AstNode::NumberLiteral(left_val), AstNode::NumberLiteral(right_val)) => match op {
                    BinaryOperator::Addition => AstNode::NumberLiteral(left_val + right_val),
                    BinaryOperator::Subtraction => AstNode::NumberLiteral(left_val - right_val),
                    BinaryOperator::Multiplication => AstNode::NumberLiteral(left_val * right_val),
                    BinaryOperator::Division => {
                        if *right_val != 0.0 {
                            AstNode::NumberLiteral(left_val / right_val)
                        } else {
                            AstNode::BinaryOp {
                                op: op.clone(),
                                left: Box::new(optimized_left),
                                right: Box::new(optimized_right),
                            }
                        }
                    }
                    BinaryOperator::Equals => {
                        AstNode::BooleanLiteral((left_val - right_val).abs() < f64::EPSILON)
                    }
                    BinaryOperator::NotEquals => {
                        AstNode::BooleanLiteral((left_val - right_val).abs() >= f64::EPSILON)
                    }
                    BinaryOperator::LessThan => AstNode::BooleanLiteral(left_val < right_val),
                    BinaryOperator::LessOrEqual => AstNode::BooleanLiteral(left_val <= right_val),
                    BinaryOperator::GreaterThan => AstNode::BooleanLiteral(left_val > right_val),
                    BinaryOperator::GreaterOrEqual => {
                        AstNode::BooleanLiteral(left_val >= right_val)
                    }
                    _ => AstNode::BinaryOp {
                        op: op.clone(),
                        left: Box::new(optimized_left),
                        right: Box::new(optimized_right),
                    },
                },
                (AstNode::StringLiteral(left_val), AstNode::StringLiteral(right_val)) => match op {
                    BinaryOperator::Equals => AstNode::BooleanLiteral(left_val == right_val),
                    BinaryOperator::NotEquals => AstNode::BooleanLiteral(left_val != right_val),
                    BinaryOperator::Addition => {
                        AstNode::StringLiteral(format!("{}{}", left_val, right_val))
                    }
                    _ => AstNode::BinaryOp {
                        op: op.clone(),
                        left: Box::new(optimized_left),
                        right: Box::new(optimized_right),
                    },
                },
                // Short-circuit optimization for boolean operations
                (AstNode::BooleanLiteral(true), _) if matches!(op, BinaryOperator::Or) => {
                    AstNode::BooleanLiteral(true)
                }
                (AstNode::BooleanLiteral(false), _) if matches!(op, BinaryOperator::And) => {
                    AstNode::BooleanLiteral(false)
                }
                (_, AstNode::BooleanLiteral(true)) if matches!(op, BinaryOperator::Or) => {
                    AstNode::BooleanLiteral(true)
                }
                (_, AstNode::BooleanLiteral(false)) if matches!(op, BinaryOperator::And) => {
                    AstNode::BooleanLiteral(false)
                }
                _ => AstNode::BinaryOp {
                    op: op.clone(),
                    left: Box::new(optimized_left),
                    right: Box::new(optimized_right),
                },
            }
        }

        // Optimize unary operations
        AstNode::UnaryOp { op, operand } => {
            let optimized_operand = optimize_ast(operand);
            match (&optimized_operand, op) {
                (AstNode::BooleanLiteral(val), UnaryOperator::Not) => AstNode::BooleanLiteral(!val),
                (AstNode::NumberLiteral(val), UnaryOperator::Negate) => {
                    AstNode::NumberLiteral(-val)
                }
                _ => AstNode::UnaryOp {
                    op: op.clone(),
                    operand: Box::new(optimized_operand),
                },
            }
        }

        // Recursively optimize path expressions
        AstNode::Path(left, right) => {
            AstNode::Path(Box::new(optimize_ast(left)), Box::new(optimize_ast(right)))
        }

        // Optimize function calls
        AstNode::FunctionCall { name, arguments } => {
            let optimized_args: Vec<AstNode> = arguments.iter().map(optimize_ast).collect();
            AstNode::FunctionCall {
                name: name.clone(),
                arguments: optimized_args,
            }
        }

        // Optimize indexing
        AstNode::Indexer { collection, index } => AstNode::Indexer {
            collection: Box::new(optimize_ast(collection)),
            index: Box::new(optimize_ast(index)),
        },

        // Literals and identifiers don't need optimization
        _ => node.clone(),
    }
}

/// Evaluates a FHIRPath expression string with a custom visitor
pub fn evaluate_expression_with_visitor(
    expression: &str,
    resource: serde_json::Value,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
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
    let result = evaluate_ast_with_visitor(&ast, &context, visitor)?;

    #[cfg(feature = "trace")]
    debug!("Expression evaluation result: {:?}", result);

    // Ensure all results are wrapped in collections as per FHIRPath specification
    let wrapped_result = match result {
        FhirPathValue::Collection(_) => result, // Already a collection
        FhirPathValue::Empty => FhirPathValue::Collection(vec![]), // Empty collection
        other => other, // Wrap single value in collection
    };

    Ok(wrapped_result)
}

/// Evaluates a FHIRPath expression string using streaming mode for large resources
pub fn evaluate_expression_streaming<R: Read>(
    expression: &str,
    reader: R,
) -> Result<FhirPathValue, FhirPathError> {
    evaluate_expression_streaming_with_visitor(expression, reader, &NoopVisitor::new())
}

/// Evaluates a FHIRPath expression string using streaming mode with a custom visitor
/// This implementation uses streaming JSON parsing to handle large resources efficiently
pub fn evaluate_expression_streaming_with_visitor<R: Read>(
    expression: &str,
    mut reader: R,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    #[cfg(feature = "trace")]
    debug!(
        "Evaluating FHIRPath expression with streaming: {}",
        expression
    );

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
    let result = evaluate_ast_with_visitor(&ast, &context, visitor)?;

    #[cfg(feature = "trace")]
    debug!("Expression evaluation result: {:?}", result);

    // Ensure all results are wrapped in collections as per FHIRPath specification
    let wrapped_result = match result {
        FhirPathValue::Collection(_) => result, // Already a collection
        FhirPathValue::Empty => FhirPathValue::Collection(vec![]), // Empty collection
        other => FhirPathValue::Collection(vec![other]), // Wrap single value in collection
    };

    Ok(wrapped_result)
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
        }
        serde_json::Value::String(s) => Ok(FhirPathValue::String(s)),
        serde_json::Value::Array(arr) => {
            let mut items = Vec::new();
            for item in arr {
                items.push(json_to_fhirpath_value(item)?);
            }
            Ok(FhirPathValue::Collection(items))
        }
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
        }
    }
}

/// Helper function for comparison operations
fn compare_values<F>(
    left: &FhirPathValue,
    right: &FhirPathValue,
    compare_fn: F,
) -> Result<FhirPathValue, FhirPathError>
where
    F: Fn(f64, f64) -> bool,
{
    // Call the internal helper with initial depth of 0
    compare_values_internal(left, right, compare_fn, 0)
}

/// Internal helper function for comparison operations with recursion depth tracking
fn compare_values_internal<F>(
    left: &FhirPathValue,
    right: &FhirPathValue,
    compare_fn: F,
    depth: usize,
) -> Result<FhirPathValue, FhirPathError>
where
    F: Fn(f64, f64) -> bool,
{
    // Prevent infinite recursion by limiting depth
    if depth > 100 {
        return Err(FhirPathError::EvaluationError(
            "Maximum recursion depth exceeded during comparison".to_string(),
        ));
    }

    match (left, right) {
        // Numeric comparisons
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a as f64, *b as f64)))
        }
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a as f64, *b)))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a, *b as f64)))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Boolean(compare_fn(*a, *b)))
        }

        // String comparisons
        (FhirPathValue::String(a), FhirPathValue::String(b)) => {
            // String comparison
            Ok(FhirPathValue::Boolean(compare_fn(
                a.cmp(b) as i32 as f64,
                0.0,
            )))
        }

        // Boolean comparisons
        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => {
            // Convert booleans to 0.0 and 1.0 for comparison
            let a_val = if *a { 1.0 } else { 0.0 };
            let b_val = if *b { 1.0 } else { 0.0 };
            Ok(FhirPathValue::Boolean(compare_fn(a_val, b_val)))
        }

        // DateTime comparisons
        (FhirPathValue::DateTime(a), FhirPathValue::DateTime(b)) => {
            // Normalize both datetimes and compare them lexicographically
            let normalized_a = normalize_datetime(a);
            let normalized_b = normalize_datetime(b);
            Ok(FhirPathValue::Boolean(compare_fn(
                normalized_a.cmp(&normalized_b) as i32 as f64,
                0.0,
            )))
        }

        // Date comparisons
        (FhirPathValue::Date(a), FhirPathValue::Date(b)) => {
            // Normalize both dates and compare them lexicographically
            let normalized_a = normalize_datetime(a);
            let normalized_b = normalize_datetime(b);
            Ok(FhirPathValue::Boolean(compare_fn(
                normalized_a.cmp(&normalized_b) as i32 as f64,
                0.0,
            )))
        }

        // Time comparisons
        (FhirPathValue::Time(a), FhirPathValue::Time(b)) => {
            // Normalize both times and compare them lexicographically
            let normalized_a = normalize_time(a);
            let normalized_b = normalize_time(b);
            Ok(FhirPathValue::Boolean(compare_fn(
                normalized_a.cmp(&normalized_b) as i32 as f64,
                0.0,
            )))
        }

        // Date to DateTime comparisons
        (FhirPathValue::Date(a), FhirPathValue::DateTime(b)) => {
            // Convert date to datetime by adding T00:00:00
            let a_as_datetime = if a.contains('T') { a.clone() } else { format!("{}T00:00:00", a) };
            let normalized_a = normalize_datetime(&a_as_datetime);
            let normalized_b = normalize_datetime(b);
            Ok(FhirPathValue::Boolean(compare_fn(
                normalized_a.cmp(&normalized_b) as i32 as f64,
                0.0,
            )))
        }
        (FhirPathValue::DateTime(a), FhirPathValue::Date(b)) => {
            // Convert date to datetime by adding T00:00:00
            let b_as_datetime = if b.contains('T') { b.clone() } else { format!("{}T00:00:00", b) };
            let normalized_a = normalize_datetime(a);
            let normalized_b = normalize_datetime(&b_as_datetime);
            Ok(FhirPathValue::Boolean(compare_fn(
                normalized_a.cmp(&normalized_b) as i32 as f64,
                0.0,
            )))
        }

        // Quantity comparisons
        (
            FhirPathValue::Quantity { value: v1, unit: u1 },
            FhirPathValue::Quantity { value: v2, unit: u2 },
        ) => {
            // For now, only compare quantities with the same unit
            if u1 == u2 {
                Ok(FhirPathValue::Boolean(compare_fn(*v1, *v2)))
            } else {
                Err(FhirPathError::TypeError(
                    "Cannot compare quantities with different units".to_string(),
                ))
            }
        }

        // Collection comparisons
        (FhirPathValue::Collection(items1), FhirPathValue::Collection(items2)) => {
            // If both collections are empty, they're equal
            if items1.is_empty() && items2.is_empty() {
                return Ok(FhirPathValue::Boolean(compare_fn(0.0, 0.0)));
            }

            // If one collection is empty and the other is not, they're not equal
            if items1.is_empty() || items2.is_empty() {
                return Ok(FhirPathValue::Boolean(compare_fn(
                    if items1.is_empty() { -1.0 } else { 1.0 },
                    0.0,
                )));
            }

            // For collections with different lengths, compare the lengths
            if items1.len() != items2.len() {
                return Ok(FhirPathValue::Boolean(compare_fn(
                    items1.len() as f64,
                    items2.len() as f64,
                )));
            }

            // For collections with the same length, compare items one by one without recursion
            // This is a non-recursive approach to avoid stack overflow
            for (i, (item1, item2)) in items1.iter().zip(items2.iter()).enumerate() {
                // Direct comparison based on value types without recursion
                let items_equal = match (item1, item2) {
                    // Simple primitive type comparisons
                    (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => a == b,
                    (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => a == b,
                    (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => a == b,
                    (FhirPathValue::String(a), FhirPathValue::String(b)) => a == b,
                    (FhirPathValue::Date(a), FhirPathValue::Date(b)) => a == b,
                    (FhirPathValue::DateTime(a), FhirPathValue::DateTime(b)) => a == b,
                    (FhirPathValue::Time(a), FhirPathValue::Time(b)) => a == b,

                    // Mixed numeric comparisons
                    (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => *a as f64 == *b,
                    (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => *a == *b as f64,

                    // Quantity comparisons
                    (
                        FhirPathValue::Quantity { value: v1, unit: u1 },
                        FhirPathValue::Quantity { value: v2, unit: u2 },
                    ) => u1 == u2 && v1 == v2,

                    // For nested collections, we can't do a deep comparison without recursion
                    // So we'll just compare if they're both collections with the same length
                    (FhirPathValue::Collection(c1), FhirPathValue::Collection(c2)) => {
                        c1.len() == c2.len()
                    },

                    // For resources, compare their JSON representations
                    (FhirPathValue::Resource(r1), FhirPathValue::Resource(r2)) => {
                        r1.to_json() == r2.to_json()
                    },

                    // Different types are not equal
                    _ => false,
                };

                if !items_equal {
                    return Ok(FhirPathValue::Boolean(compare_fn(1.0, 0.0)));
                }
            }

            // If all items are equal, the collections are equal
            Ok(FhirPathValue::Boolean(compare_fn(0.0, 0.0)))
        }

        // String to number conversions for comparison
        (FhirPathValue::String(s), FhirPathValue::Integer(i)) => {
            if let Ok(s_as_num) = s.parse::<f64>() {
                Ok(FhirPathValue::Boolean(compare_fn(s_as_num, *i as f64)))
            } else {
                Err(FhirPathError::TypeError(
                    "Cannot compare string to number".to_string(),
                ))
            }
        }
        (FhirPathValue::Integer(i), FhirPathValue::String(s)) => {
            if let Ok(s_as_num) = s.parse::<f64>() {
                Ok(FhirPathValue::Boolean(compare_fn(*i as f64, s_as_num)))
            } else {
                Err(FhirPathError::TypeError(
                    "Cannot compare number to string".to_string(),
                ))
            }
        }
        (FhirPathValue::String(s), FhirPathValue::Decimal(d)) => {
            if let Ok(s_as_num) = s.parse::<f64>() {
                Ok(FhirPathValue::Boolean(compare_fn(s_as_num, *d)))
            } else {
                Err(FhirPathError::TypeError(
                    "Cannot compare string to decimal".to_string(),
                ))
            }
        }
        (FhirPathValue::Decimal(d), FhirPathValue::String(s)) => {
            if let Ok(s_as_num) = s.parse::<f64>() {
                Ok(FhirPathValue::Boolean(compare_fn(*d, s_as_num)))
            } else {
                Err(FhirPathError::TypeError(
                    "Cannot compare decimal to string".to_string(),
                ))
            }
        }

        // Empty value comparisons
        (FhirPathValue::Empty, _) | (_, FhirPathValue::Empty) => {
            // Empty values are considered less than any other value
            Ok(FhirPathValue::Boolean(compare_fn(-1.0, 0.0)))
        }

        // Fallback for incompatible types
        _ => Err(FhirPathError::TypeError(
            format!("Comparison requires compatible operands: {:?} and {:?}", left, right),
        )),
    }
}

/// Helper function for addition
fn add_values(left: &FhirPathValue, right: &FhirPathValue) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => Ok(FhirPathValue::Integer(a + b)),
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 + b))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a + *b as f64))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => Ok(FhirPathValue::Decimal(a + b)),
        (FhirPathValue::String(a), FhirPathValue::String(b)) => {
            // String concatenation
            Ok(FhirPathValue::String(format!("{}{}", a, b)))
        }
        (FhirPathValue::Collection(a), FhirPathValue::Collection(b)) => {
            // Collection union
            let mut result = a.clone();
            result.extend(b.clone());
            Ok(FhirPathValue::Collection(result))
        }
        _ => Err(FhirPathError::TypeError(
            "Addition requires compatible operands".to_string(),
        )),
    }
}

/// Helper function for subtraction
fn subtract_values(
    left: &FhirPathValue,
    right: &FhirPathValue,
) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => Ok(FhirPathValue::Integer(a - b)),
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 - b))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a - *b as f64))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => Ok(FhirPathValue::Decimal(a - b)),
        _ => Err(FhirPathError::TypeError(
            "Subtraction requires numeric operands".to_string(),
        )),
    }
}

/// Helper function for multiplication
fn multiply_values(
    left: &FhirPathValue,
    right: &FhirPathValue,
) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => Ok(FhirPathValue::Integer(a * b)),
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 * b))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a * *b as f64))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => Ok(FhirPathValue::Decimal(a * b)),
        _ => Err(FhirPathError::TypeError(
            "Multiplication requires numeric operands".to_string(),
        )),
    }
}

/// Helper function for division
fn divide_values(
    left: &FhirPathValue,
    right: &FhirPathValue,
) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (_, FhirPathValue::Integer(b)) if *b == 0 => Err(FhirPathError::EvaluationError(
            "Division by zero".to_string(),
        )),
        (_, FhirPathValue::Decimal(b)) if *b == 0.0 => Err(FhirPathError::EvaluationError(
            "Division by zero".to_string(),
        )),
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            // Integer division results in a decimal
            Ok(FhirPathValue::Decimal(*a as f64 / *b as f64))
        }
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(*a as f64 / b))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a / *b as f64))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => Ok(FhirPathValue::Decimal(a / b)),
        _ => Err(FhirPathError::TypeError(
            "Division requires numeric operands".to_string(),
        )),
    }
}

/// Helper function for modulo operation
fn mod_values(
    left: &FhirPathValue,
    right: &FhirPathValue,
) -> Result<FhirPathValue, FhirPathError> {
    match (left, right) {
        (_, FhirPathValue::Integer(b)) if *b == 0 => Err(FhirPathError::EvaluationError(
            "Modulo by zero".to_string(),
        )),
        (_, FhirPathValue::Decimal(b)) if *b == 0.0 => Err(FhirPathError::EvaluationError(
            "Modulo by zero".to_string(),
        )),
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Integer(a % b))
        }
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal((*a as f64) % b))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            Ok(FhirPathValue::Decimal(a % (*b as f64)))
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => {
            Ok(FhirPathValue::Decimal(a % b))
        }
        _ => Err(FhirPathError::TypeError(
            "Modulo requires numeric operands".to_string(),
        )),
    }
}

/// Evaluates a function call with proper argument handling
fn evaluate_function_call(
    name: &str,
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
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
        "subsetOf" => evaluate_subset_of_function(arguments, context, visitor),

        // Type checking functions
        "is" => evaluate_is_function(arguments, context),
        "as" => evaluate_as_function(arguments, context),

        // String functions
        "contains" => evaluate_contains_function(arguments, context),
        "startsWith" => evaluate_starts_with_function(arguments, context),
        "endsWith" => evaluate_ends_with_function(arguments, context),
        "substring" => evaluate_substring_function(arguments, context, visitor),
        "indexOf" => evaluate_index_of_function(arguments, context),
        "replace" => evaluate_replace_function(arguments, context),
        "matches" => evaluate_matches_function(arguments, context),
        "split" => evaluate_split_function(arguments, context, visitor),
        "join" => evaluate_join_function(arguments, context, visitor),

        // Math functions
        "abs" => evaluate_abs_function(arguments, context, visitor),
        "ceiling" => evaluate_ceiling_function(arguments, context, visitor),
        "floor" => evaluate_floor_function(arguments, context, visitor),
        "round" => evaluate_round_function(arguments, context, visitor),
        "sqrt" => evaluate_sqrt_function(arguments, context, visitor),
        "exp" => evaluate_exp_function(arguments, context, visitor),
        "ln" => evaluate_ln_function(arguments, context, visitor),
        "log" => evaluate_log_function(arguments, context, visitor),
        "power" => evaluate_power_function(arguments, context, visitor),
        "truncate" => evaluate_truncate_function(arguments, context, visitor),

        // Date/time functions
        "now" => evaluate_now_function(arguments, context),
        "today" => evaluate_today_function(arguments, context),
        "timeOfDay" => evaluate_time_of_day_function(arguments, context),

        // Boolean functions
        "not" => evaluate_not_function(arguments, context, visitor),
        "all" => evaluate_all_function(arguments, context, visitor),
        "allTrue" => evaluate_all_true_function(arguments, context, visitor),
        "anyTrue" => evaluate_any_true_function(arguments, context, visitor),
        "allFalse" => evaluate_all_false_function(arguments, context, visitor),
        "anyFalse" => evaluate_any_false_function(arguments, context, visitor),

        // Conversion functions
        "convertsToInteger" => evaluate_converts_to_integer_function(arguments, context, visitor),
        "convertsToString" => evaluate_converts_to_string_function(arguments, context, visitor),
        "convertsToBoolean" => evaluate_converts_to_boolean_function(arguments, context, visitor),
        "convertsToDecimal" => evaluate_converts_to_decimal_function(arguments, context, visitor),
        "convertsToDate" => evaluate_converts_to_date_function(arguments, context, visitor),
        "convertsToDateTime" => {
            evaluate_converts_to_date_time_function(arguments, context, visitor)
        }
        "convertsToQuantity" => evaluate_converts_to_quantity_function(arguments, context, visitor),

        // Type and metadata functions
        "type" => evaluate_type_function(arguments, context, visitor),
        "extension" => evaluate_extension_function(arguments, context, visitor),
        "ofType" => evaluate_of_type_function(arguments, context, visitor),
        "conformsTo" => evaluate_conforms_to_function(arguments, context, visitor),

        _ => Err(FhirPathError::EvaluationError(format!(
            "Unknown function: {}",
            name
        ))),
    }
}

/// Evaluates the where() function for filtering collections
fn evaluate_where_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'where' function expects 1 argument, got {}",
            arguments.len()
        )));
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
                let filter_result =
                    evaluate_ast_with_visitor(&arguments[0], &item_context, visitor)?;

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
fn evaluate_select_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'select' function expects 1 argument, got {}",
            arguments.len()
        )));
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
                }
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
fn evaluate_first_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'first' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    let collection = get_current_collection(context)?;
    if collection.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(collection[0].clone())
    }
}

/// Evaluates the last() function
fn evaluate_last_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'last' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    let collection = get_current_collection(context)?;
    if collection.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(collection[collection.len() - 1].clone())
    }
}

/// Evaluates the tail() function
fn evaluate_tail_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'tail' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    let collection = get_current_collection(context)?;
    if collection.len() <= 1 {
        Ok(FhirPathValue::Empty)
    } else {
        // Memory optimization: for large collections, avoid creating new vectors
        if context.optimization_enabled && collection.len() > 1000 {
            // For large collections, create a lazy slice
            let mut result = Vec::with_capacity(collection.len() - 1);
            result.extend_from_slice(&collection[1..]);
            Ok(FhirPathValue::Collection(result))
        } else {
            Ok(FhirPathValue::Collection(collection[1..].to_vec()))
        }
    }
}

/// Evaluates the skip() function
fn evaluate_skip_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'skip' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    let skip_count_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
    let skip_count = match skip_count_result {
        FhirPathValue::Integer(n) => n as usize,
        _ => {
            return Err(FhirPathError::TypeError(
                "'skip' function requires an integer argument".to_string(),
            ));
        }
    };

    let collection = get_current_collection(context)?;
    if skip_count >= collection.len() {
        Ok(FhirPathValue::Empty)
    } else {
        // Memory optimization: for large collections, use iterator-based approach
        if context.optimization_enabled && collection.len() > 1000 {
            let result: Vec<FhirPathValue> = collection.iter().skip(skip_count).cloned().collect();
            Ok(FhirPathValue::Collection(result))
        } else {
            Ok(FhirPathValue::Collection(collection[skip_count..].to_vec()))
        }
    }
}

/// Evaluates the take() function
fn evaluate_take_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'take' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    let take_count_result = evaluate_ast(&arguments[0], context)?;
    let take_count = match take_count_result {
        FhirPathValue::Integer(n) => n as usize,
        _ => {
            return Err(FhirPathError::TypeError(
                "'take' function requires an integer argument".to_string(),
            ));
        }
    };

    let collection = get_current_collection(context)?;
    let end_index = std::cmp::min(take_count, collection.len());

    if end_index == 0 {
        Ok(FhirPathValue::Empty)
    } else {
        // Memory optimization: for large collections, use iterator-based approach
        if context.optimization_enabled && collection.len() > 1000 {
            let result: Vec<FhirPathValue> = collection.iter().take(end_index).cloned().collect();
            Ok(FhirPathValue::Collection(result))
        } else {
            Ok(FhirPathValue::Collection(collection[..end_index].to_vec()))
        }
    }
}

/// Evaluates the exists() function
fn evaluate_exists_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() > 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'exists' function expects 0 or 1 arguments, got {}",
            arguments.len()
        )));
    }
    if arguments.is_empty() {
        // Check if the current context has any values
        match get_current_collection(context) {
            Ok(collection) => {
                if collection.len() == 1 && collection[0] == FhirPathValue::Empty {
                    return Ok(FhirPathValue::Boolean(false))
                }
                Ok(FhirPathValue::Boolean(!collection.is_empty()))
            },
            Err(_) => Ok(FhirPathValue::Boolean(false)),
        }
    } else {
        // Check if any item in the collection satisfies the condition
        let collection = get_current_collection(context)?;
        let total = collection.len();

        for (idx, item) in collection.into_iter().enumerate() {
            let item_context = context.create_iteration_context(item, idx, total)?;
            let condition_result = evaluate_ast(&arguments[0], &item_context)?;
            println!("condition_result: {:?}", condition_result);
            if is_truthy(&condition_result) {
                return Ok(FhirPathValue::Boolean(true));
            }
        }

        Ok(FhirPathValue::Boolean(false))
    }
}

/// Evaluates the empty() function
fn evaluate_empty_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'empty' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    match get_current_collection(context) {
        Ok(collection) => Ok(FhirPathValue::Boolean(collection.is_empty())),
        Err(_) => Ok(FhirPathValue::Boolean(true)),
    }
}

/// Evaluates the count() function
fn evaluate_count_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'count' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    let collection = get_current_collection(context)?;
    Ok(FhirPathValue::Integer(collection.len() as i64))
}

/// Evaluates the length() function for strings
fn evaluate_length_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'length' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    // Get the current value from context
    match &context.context {
        serde_json::Value::String(s) => Ok(FhirPathValue::Integer(s.len() as i64)),
        _ => Err(FhirPathError::TypeError(
            "'length' function can only be applied to strings".to_string(),
        )),
    }
}

/// Evaluates the distinct() function
fn evaluate_distinct_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'distinct' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    let collection = get_current_collection(context)?;
    let mut unique_items = Vec::new();

    for item in collection {
        if !unique_items
            .iter()
            .any(|existing| values_equal(existing, &item))
        {
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
fn evaluate_union_function(
    _arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented(
        "'union' function not yet implemented".to_string(),
    ))
}

fn evaluate_intersect_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'intersect' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Evaluate the argument to get the other collection
    let visitor = NoopVisitor::new();
    let other_result = evaluate_ast_with_visitor(&arguments[0], context, &visitor)?;
    let other_collection = match other_result {
        FhirPathValue::Collection(items) => items,
        FhirPathValue::Empty => vec![],
        single_item => vec![single_item],
    };

    // Find intersection - items that exist in both collections
    let mut intersection_items = Vec::new();

    for current_item in &current_collection {
        // Check if this item exists in the other collection
        let mut found_in_other = false;
        for other_item in &other_collection {
            if values_equal(current_item, other_item) {
                found_in_other = true;
                break;
            }
        }

        // If found in other collection, add to intersection (avoiding duplicates)
        if found_in_other {
            let mut already_in_intersection = false;
            for existing_item in &intersection_items {
                if values_equal(current_item, existing_item) {
                    already_in_intersection = true;
                    break;
                }
            }
            if !already_in_intersection {
                intersection_items.push(current_item.clone());
            }
        }
    }

    if intersection_items.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(intersection_items))
    }
}

fn evaluate_subset_of_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'subsetOf' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Evaluate the argument to get the comparison collection
    let comparison_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
    let comparison_collection = match comparison_result {
        FhirPathValue::Collection(items) => items,
        FhirPathValue::Empty => vec![],
        single_item => vec![single_item],
    };

    // Check if all items in current collection exist in comparison collection
    for current_item in &current_collection {
        let mut found = false;
        for comparison_item in &comparison_collection {
            if values_equal(current_item, comparison_item) {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(FhirPathValue::Boolean(false));
        }
    }

    Ok(FhirPathValue::Boolean(true))
}

fn evaluate_is_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'is' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Extract type name from the argument - handle both identifiers and path expressions
    let type_name = match &arguments[0] {
        AstNode::Identifier(name) => name.clone(),
        AstNode::Path(left, right) => {
            // Handle path expressions like System.Boolean
            match (left.as_ref(), right.as_ref()) {
                (AstNode::Identifier(namespace), AstNode::Identifier(type_name)) => {
                    format!("{}.{}", namespace, type_name)
                }
                _ => {
                    return Err(FhirPathError::EvaluationError(
                        "'is' function expects a type name or qualified type name as argument".to_string(),
                    ))
                }
            }
        }
        _ => {
            return Err(FhirPathError::EvaluationError(
                "'is' function expects a type name or qualified type name as argument".to_string(),
            ))
        }
    };

    // Check if any item in the current collection matches the specified type
    for item in &current_collection {
        let matches_type = match (item, type_name.as_str()) {
            // System types (both capitalized and lowercase)
            (FhirPathValue::String(_), "String" | "string" | "System.String") => true,
            (FhirPathValue::Integer(_), "Integer" | "integer" | "System.Integer") => true,
            (FhirPathValue::Decimal(_), "Decimal" | "decimal" | "System.Decimal") => true,
            (FhirPathValue::Boolean(_), "Boolean" | "boolean" | "System.Boolean") => true,
            (FhirPathValue::Date(_), "Date" | "date" | "System.Date") => true,
            (FhirPathValue::DateTime(_), "DateTime" | "dateTime" | "System.DateTime") => true,
            (FhirPathValue::Time(_), "Time" | "time" | "System.Time") => true,
            (FhirPathValue::Quantity { .. }, "Quantity" | "System.Quantity") => true,
            (FhirPathValue::Collection(_), "Collection" | "System.Collection") => true,

            // FHIR primitive types - these should be treated as FHIR types, not System types
            (FhirPathValue::Boolean(_), "FHIR.boolean") => true,
            (FhirPathValue::String(_), "FHIR.string") => true,
            (FhirPathValue::Integer(_), "FHIR.integer") => true,
            (FhirPathValue::Decimal(_), "FHIR.decimal") => true,
            (FhirPathValue::Date(_), "FHIR.date") => true,
            (FhirPathValue::DateTime(_), "FHIR.dateTime") => true,
            (FhirPathValue::Time(_), "FHIR.time") => true,

            // FHIR resource types
            (FhirPathValue::Resource(resource), type_name) => {
                if let Some(resource_type) = &resource.resource_type {
                    // Check exact match or FHIR-qualified match
                    resource_type == type_name || format!("FHIR.{}", resource_type) == type_name
                } else {
                    // Generic resource type check
                    type_name == "Resource" || type_name == "resource" || type_name == "FHIR.Resource"
                }
            }
            _ => false,
        };

        if matches_type {
            return Ok(FhirPathValue::Boolean(true));
        }
    }

    Ok(FhirPathValue::Boolean(false))
}

fn evaluate_as_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'as' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Get the type name from the argument
    let type_name = match &arguments[0] {
        AstNode::Identifier(name) => name.clone(),
        _ => {
            return Err(FhirPathError::TypeError(
                "'as' function requires a type identifier".to_string(),
            ))
        }
    };

    let mut results = Vec::new();

    for item in &current_collection {
        // First try direct type matching
        let matches_type = match (item, type_name.as_str()) {
            (FhirPathValue::String(_), "string") => true,
            (FhirPathValue::Integer(_), "integer") => true,
            (FhirPathValue::Decimal(_), "decimal") => true,
            (FhirPathValue::Boolean(_), "boolean") => true,
            (FhirPathValue::Date(_), "date") => true,
            (FhirPathValue::DateTime(_), "dateTime") => true,
            (FhirPathValue::Time(_), "time") => true,
            (FhirPathValue::Time(_), "Time") => true,
            (FhirPathValue::Quantity { .. }, "Quantity") => true,
            // For FHIR resource types, check if the resource has the expected resourceType
            (FhirPathValue::Resource(resource), type_name) => {
                if let Some(resource_type) = &resource.resource_type {
                    resource_type == type_name
                } else {
                    false
                }
            }
            _ => false,
        };

        if matches_type {
            results.push(item.clone());
            continue;
        }

        // If direct type matching fails, try conversion
        let converted_value = match (item, type_name.as_str()) {
            // String to DateTime/Date/Time conversion
            (FhirPathValue::String(s), "dateTime") |
            (FhirPathValue::String(s), "date") |
            (FhirPathValue::String(s), "time") => {
                if let Some(dt_value) = string_to_datetime(s) {
                    // Only add if the converted type matches the requested type
                    match (dt_value.clone(), type_name.as_str()) {
                        (FhirPathValue::DateTime(_), "dateTime") |
                        (FhirPathValue::Date(_), "date") |
                        (FhirPathValue::Time(_), "time") => Some(dt_value),
                        _ => None
                    }
                } else {
                    None
                }
            },
            // String to Integer conversion
            (FhirPathValue::String(s), "integer") => {
                s.parse::<i64>().ok().map(FhirPathValue::Integer)
            },
            // String to Decimal conversion
            (FhirPathValue::String(s), "decimal") => {
                s.parse::<f64>().ok().map(FhirPathValue::Decimal)
            },
            // String to Boolean conversion
            (FhirPathValue::String(s), "boolean") => {
                match s.to_lowercase().as_str() {
                    "true" => Some(FhirPathValue::Boolean(true)),
                    "false" => Some(FhirPathValue::Boolean(false)),
                    _ => None
                }
            },
            // Integer to Decimal conversion
            (FhirPathValue::Integer(i), "decimal") => {
                Some(FhirPathValue::Decimal(*i as f64))
            },
            // Decimal to Integer conversion (truncates)
            (FhirPathValue::Decimal(d), "integer") => {
                Some(FhirPathValue::Integer(*d as i64))
            },
            _ => None
        };

        if let Some(value) = converted_value {
            results.push(value);
        }
        // If conversion fails, we don't add anything to results
    }

    if results.is_empty() {
        Ok(FhirPathValue::Empty)
    } else if results.len() == 1 {
        Ok(results.into_iter().next().unwrap())
    } else {
        Ok(FhirPathValue::Collection(results))
    }
}

fn evaluate_contains_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'contains' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Evaluate the substring argument
    let substring_result =
        evaluate_ast_internal_uncached(&arguments[0], context, &NoopVisitor::new())?;

    let substring = match substring_result {
        FhirPathValue::String(s) => s,
        _ => {
            return Err(FhirPathError::TypeError(
                "'contains' function requires a string argument".to_string(),
            ))
        }
    };

    // Check if any string in the current collection contains the substring
    for item in &current_collection {
        if let FhirPathValue::String(s) = item {
            if s.contains(&substring) {
                return Ok(FhirPathValue::Boolean(true));
            }
        }
    }

    Ok(FhirPathValue::Boolean(false))
}

fn evaluate_starts_with_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'startsWith' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Evaluate the prefix argument
    let prefix_result =
        evaluate_ast_internal_uncached(&arguments[0], context, &NoopVisitor::new())?;

    let prefix = match prefix_result {
        FhirPathValue::String(s) => s,
        _ => {
            return Err(FhirPathError::TypeError(
                "'startsWith' function requires a string argument".to_string(),
            ))
        }
    };

    // Check if any string in the current collection starts with the prefix
    for item in &current_collection {
        if let FhirPathValue::String(s) = item {
            if s.starts_with(&prefix) {
                return Ok(FhirPathValue::Boolean(true));
            }
        }
    }

    Ok(FhirPathValue::Boolean(false))
}

fn evaluate_ends_with_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'endsWith' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let current_collection = get_current_collection(context)?;

    // Evaluate the suffix argument
    let suffix_result =
        evaluate_ast_internal_uncached(&arguments[0], context, &NoopVisitor::new())?;

    let suffix = match suffix_result {
        FhirPathValue::String(s) => s,
        _ => {
            return Err(FhirPathError::TypeError(
                "'endsWith' function requires a string argument".to_string(),
            ))
        }
    };

    // Check if any string in the current collection ends with the suffix
    for item in &current_collection {
        if let FhirPathValue::String(s) = item {
            if s.ends_with(&suffix) {
                return Ok(FhirPathValue::Boolean(true));
            }
        }
    }

    Ok(FhirPathValue::Boolean(false))
}

fn evaluate_substring_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() < 1 || arguments.len() > 2 {
        return Err(FhirPathError::EvaluationError(format!(
            "'substring' function expects 1 or 2 arguments, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    for item in collection {
        if let FhirPathValue::String(s) = item {
            let start_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

            if let FhirPathValue::Integer(start) = start_result {
                let start_idx = if start < 0 { 0 } else { start as usize };

                if arguments.len() == 2 {
                    let length_result = evaluate_ast_with_visitor(&arguments[1], context, visitor)?;
                    if let FhirPathValue::Integer(length) = length_result {
                        if length <= 0 {
                            return Ok(FhirPathValue::String("".to_string()));
                        }
                        let _end_idx = start_idx + (length as usize);
                        let result = if start_idx >= s.len() {
                            "".to_string()
                        } else {
                            s.chars().skip(start_idx).take(length as usize).collect()
                        };
                        return Ok(FhirPathValue::String(result));
                    } else {
                        return Err(FhirPathError::TypeError(
                            "'substring' function length argument must be an integer".to_string(),
                        ));
                    }
                } else {
                    // Only start index provided, return substring from start to end
                    let result = if start_idx >= s.len() {
                        "".to_string()
                    } else {
                        s.chars().skip(start_idx).collect()
                    };
                    return Ok(FhirPathValue::String(result));
                }
            } else {
                return Err(FhirPathError::TypeError(
                    "'substring' function start argument must be an integer".to_string(),
                ));
            }
        }
    }

    Ok(FhirPathValue::Empty)
}

fn evaluate_index_of_function(
    _arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented(
        "'indexOf' function not yet implemented".to_string(),
    ))
}

fn evaluate_replace_function(
    _arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented(
        "'replace' function not yet implemented".to_string(),
    ))
}

fn evaluate_matches_function(
    _arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented(
        "'matches' function not yet implemented".to_string(),
    ))
}

fn evaluate_split_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'split' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    for item in collection {
        if let FhirPathValue::String(s) = item {
            let delimiter_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

            if let FhirPathValue::String(delimiter) = delimiter_result {
                let parts: Vec<FhirPathValue> = s
                    .split(&delimiter)
                    .map(|part| FhirPathValue::String(part.to_string()))
                    .collect();

                return Ok(FhirPathValue::Collection(parts));
            } else {
                return Err(FhirPathError::TypeError(
                    "'split' function delimiter argument must be a string".to_string(),
                ));
            }
        }
    }

    Ok(FhirPathValue::Empty)
}

fn evaluate_join_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'join' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    // Evaluate the separator argument
    let separator_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

    let separator = match separator_result {
        FhirPathValue::String(s) => s,
        _ => return Err(FhirPathError::TypeError(
            "'join' function separator argument must be a string".to_string(),
        )),
    };

    // Handle empty collection case
    if collection.is_empty() {
        return Ok(FhirPathValue::String(String::new()));
    }

    // Collect all string values from the collection
    let mut string_values = Vec::new();
    for item in collection {
        match item {
            FhirPathValue::String(s) => string_values.push(s),
            // Skip non-string values instead of erroring - this is more consistent with FHIRPath behavior
            _ => continue,
        }
    }

    // Join the strings with the separator
    let joined = string_values.join(&separator);
    Ok(FhirPathValue::String(joined))
}

fn evaluate_abs_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i.abs())),
                FhirPathValue::Decimal(d) => results.push(FhirPathValue::Decimal(d.abs())),
                _ => return Err(FhirPathError::TypeError(
                    "'abs' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => FhirPathValue::Integer(i.abs()),
            FhirPathValue::Decimal(d) => FhirPathValue::Decimal(d.abs()),
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i.abs())),
                        FhirPathValue::Decimal(d) => results.push(FhirPathValue::Decimal(d.abs())),
                        _ => return Err(FhirPathError::TypeError(
                            "'abs' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'abs' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'abs' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_ceiling_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.ceil() as i64)),
                _ => return Err(FhirPathError::TypeError(
                    "'ceiling' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => FhirPathValue::Integer(i),
            FhirPathValue::Decimal(d) => FhirPathValue::Integer(d.ceil() as i64),
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                        FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.ceil() as i64)),
                        _ => return Err(FhirPathError::TypeError(
                            "'ceiling' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'ceiling' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'ceiling' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_floor_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.floor() as i64)),
                _ => return Err(FhirPathError::TypeError(
                    "'floor' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => FhirPathValue::Integer(i),
            FhirPathValue::Decimal(d) => FhirPathValue::Integer(d.floor() as i64),
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                        FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.floor() as i64)),
                        _ => return Err(FhirPathError::TypeError(
                            "'floor' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'floor' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'floor' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_round_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.round() as i64)),
                _ => return Err(FhirPathError::TypeError(
                    "'round' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => FhirPathValue::Integer(i),
            FhirPathValue::Decimal(d) => FhirPathValue::Integer(d.round() as i64),
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                        FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.round() as i64)),
                        _ => return Err(FhirPathError::TypeError(
                            "'round' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'round' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'round' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_sqrt_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => {
                    if i < 0 {
                        return Err(FhirPathError::EvaluationError(
                            "Cannot take square root of negative number".to_string(),
                        ));
                    } else {
                        results.push(FhirPathValue::Decimal((i as f64).sqrt()));
                    }
                }
                FhirPathValue::Decimal(d) => {
                    if d < 0.0 {
                        return Err(FhirPathError::EvaluationError(
                            "Cannot take square root of negative number".to_string(),
                        ));
                    } else {
                        results.push(FhirPathValue::Decimal(d.sqrt()));
                    }
                }
                _ => return Err(FhirPathError::TypeError(
                    "'sqrt' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => {
                if i < 0 {
                    return Err(FhirPathError::EvaluationError(
                        "Cannot take square root of negative number".to_string(),
                    ));
                } else {
                    FhirPathValue::Decimal((i as f64).sqrt())
                }
            }
            FhirPathValue::Decimal(d) => {
                if d < 0.0 {
                    return Err(FhirPathError::EvaluationError(
                        "Cannot take square root of negative number".to_string(),
                    ));
                } else {
                    FhirPathValue::Decimal(d.sqrt())
                }
            }
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => {
                            if i < 0 {
                                return Err(FhirPathError::EvaluationError(
                                    "Cannot take square root of negative number".to_string(),
                                ));
                            } else {
                                results.push(FhirPathValue::Decimal((i as f64).sqrt()));
                            }
                        }
                        FhirPathValue::Decimal(d) => {
                            if d < 0.0 {
                                return Err(FhirPathError::EvaluationError(
                                    "Cannot take square root of negative number".to_string(),
                                ));
                            } else {
                                results.push(FhirPathValue::Decimal(d.sqrt()));
                            }
                        }
                        _ => return Err(FhirPathError::TypeError(
                            "'sqrt' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'sqrt' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'sqrt' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_exp_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => results.push(FhirPathValue::Decimal((i as f64).exp())),
                FhirPathValue::Decimal(d) => results.push(FhirPathValue::Decimal(d.exp())),
                _ => return Err(FhirPathError::TypeError(
                    "'exp' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => FhirPathValue::Decimal((i as f64).exp()),
            FhirPathValue::Decimal(d) => FhirPathValue::Decimal(d.exp()),
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => results.push(FhirPathValue::Decimal((i as f64).exp())),
                        FhirPathValue::Decimal(d) => results.push(FhirPathValue::Decimal(d.exp())),
                        _ => return Err(FhirPathError::TypeError(
                            "'exp' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'exp' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'exp' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_ln_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => {
                    if i <= 0 {
                        return Err(FhirPathError::EvaluationError(
                            "Cannot take natural log of non-positive number".to_string(),
                        ));
                    } else {
                        results.push(FhirPathValue::Decimal((i as f64).ln()));
                    }
                }
                FhirPathValue::Decimal(d) => {
                    if d <= 0.0 {
                        return Err(FhirPathError::EvaluationError(
                            "Cannot take natural log of non-positive number".to_string(),
                        ));
                    } else {
                        results.push(FhirPathValue::Decimal(d.ln()));
                    }
                }
                _ => return Err(FhirPathError::TypeError(
                    "'ln' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => {
                if i <= 0 {
                    return Err(FhirPathError::EvaluationError(
                        "Cannot take natural log of non-positive number".to_string(),
                    ));
                } else {
                    FhirPathValue::Decimal((i as f64).ln())
                }
            }
            FhirPathValue::Decimal(d) => {
                if d <= 0.0 {
                    return Err(FhirPathError::EvaluationError(
                        "Cannot take natural log of non-positive number".to_string(),
                    ));
                } else {
                    FhirPathValue::Decimal(d.ln())
                }
            }
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => {
                            if i <= 0 {
                                return Err(FhirPathError::EvaluationError(
                                    "Cannot take natural log of non-positive number".to_string(),
                                ));
                            } else {
                                results.push(FhirPathValue::Decimal((i as f64).ln()));
                            }
                        }
                        FhirPathValue::Decimal(d) => {
                            if d <= 0.0 {
                                return Err(FhirPathError::EvaluationError(
                                    "Cannot take natural log of non-positive number".to_string(),
                                ));
                            } else {
                                results.push(FhirPathValue::Decimal(d.ln()));
                            }
                        }
                        _ => return Err(FhirPathError::TypeError(
                            "'ln' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'ln' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'ln' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_log_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let (value, base) = if arguments.len() == 1 {
        // Method call syntax: value.log(base)
        // Use this_item as value and first argument as base
        if let Some(this_item) = &context.this_item {
            let value = match this_item {
                FhirPathValue::Collection(items) if items.len() == 1 => items[0].clone(),
                FhirPathValue::Collection(_) => {
                    return Err(FhirPathError::EvaluationError(
                        "'log' function cannot be applied to collections with multiple items".to_string(),
                    ));
                }
                other => other.clone(),
            };
            let base = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
            (value, base)
        } else {
            return Err(FhirPathError::EvaluationError(
                "'log' function expects method call syntax with base argument".to_string(),
            ));
        }
    } else if arguments.len() == 2 {
        // Function call syntax: log(value, base)
        let value = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
        let base = evaluate_ast_with_visitor(&arguments[1], context, visitor)?;
        (value, base)
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'log' function expects 1 or 2 arguments, got {}",
            arguments.len()
        )));
    };

    let (value_f64, base_f64) = match (value, base) {
        (FhirPathValue::Integer(v), FhirPathValue::Integer(b)) => (v as f64, b as f64),
        (FhirPathValue::Integer(v), FhirPathValue::Decimal(b)) => (v as f64, b),
        (FhirPathValue::Decimal(v), FhirPathValue::Integer(b)) => (v, b as f64),
        (FhirPathValue::Decimal(v), FhirPathValue::Decimal(b)) => (v, b),
        _ => return Err(FhirPathError::TypeError(
            "'log' function can only be applied to numbers".to_string(),
        )),
    };

    if value_f64 <= 0.0 {
        return Err(FhirPathError::EvaluationError(
            "Cannot take log of non-positive number".to_string(),
        ));
    }

    if base_f64 <= 0.0 || base_f64 == 1.0 {
        return Err(FhirPathError::EvaluationError(
            "Log base must be positive and not equal to 1".to_string(),
        ));
    }

    // Calculate log_base(value) = ln(value) / ln(base)
    let result = value_f64.ln() / base_f64.ln();
    Ok(FhirPathValue::Decimal(result))
}

fn evaluate_power_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let (base, exponent) = if arguments.len() == 1 {
        // Method call syntax: value.power(exponent)
        // Use this_item as base and first argument as exponent
        if let Some(this_item) = &context.this_item {
            let base = match this_item {
                FhirPathValue::Collection(items) if items.len() == 1 => items[0].clone(),
                FhirPathValue::Collection(_) => {
                    return Err(FhirPathError::EvaluationError(
                        "'power' function cannot be applied to collections with multiple items".to_string(),
                    ));
                }
                other => other.clone(),
            };
            let exponent = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
            (base, exponent)
        } else {
            return Err(FhirPathError::EvaluationError(
                "'power' function expects 2 arguments or method call syntax".to_string(),
            ));
        }
    } else if arguments.len() == 2 {
        // Function call syntax: power(base, exponent)
        let base = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;
        let exponent = evaluate_ast_with_visitor(&arguments[1], context, visitor)?;
        (base, exponent)
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'power' function expects 1 or 2 arguments, got {}",
            arguments.len()
        )));
    };

    match (base, exponent) {
        (FhirPathValue::Integer(b), FhirPathValue::Integer(e)) => {
            Ok(FhirPathValue::Decimal((b as f64).powf(e as f64)))
        }
        (FhirPathValue::Integer(b), FhirPathValue::Decimal(e)) => {
            Ok(FhirPathValue::Decimal((b as f64).powf(e)))
        }
        (FhirPathValue::Decimal(b), FhirPathValue::Integer(e)) => {
            Ok(FhirPathValue::Decimal(b.powf(e as f64)))
        }
        (FhirPathValue::Decimal(b), FhirPathValue::Decimal(e)) => {
            Ok(FhirPathValue::Decimal(b.powf(e)))
        }
        _ => Err(FhirPathError::TypeError(
            "'power' function can only be applied to numbers".to_string(),
        )),
    }
}

fn evaluate_truncate_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    // If no arguments, apply to the current collection
    let result = if arguments.is_empty() {
        // Get the current collection from context
        let collection = get_current_collection(context)?;
        let mut results = Vec::new();

        for item in collection {
            match item {
                FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.trunc() as i64)),
                _ => return Err(FhirPathError::TypeError(
                    "'truncate' function can only be applied to numbers".to_string(),
                )),
            }
        }

        if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            FhirPathValue::Collection(results)
        }
    } else if arguments.len() == 1 {
        let result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

        match result {
            FhirPathValue::Integer(i) => FhirPathValue::Integer(i),
            FhirPathValue::Decimal(d) => FhirPathValue::Integer(d.trunc() as i64),
            FhirPathValue::Collection(items) => {
                let mut results = Vec::new();
                for item in items {
                    match item {
                        FhirPathValue::Integer(i) => results.push(FhirPathValue::Integer(i)),
                        FhirPathValue::Decimal(d) => results.push(FhirPathValue::Integer(d.trunc() as i64)),
                        _ => return Err(FhirPathError::TypeError(
                            "'truncate' function can only be applied to numbers".to_string(),
                        )),
                    }
                }
                FhirPathValue::Collection(results)
            },
            _ => return Err(FhirPathError::TypeError(
                "'truncate' function can only be applied to numbers".to_string(),
            )),
        }
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'truncate' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    Ok(result)
}

fn evaluate_type_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let result = if arguments.is_empty() {
        // Method call syntax: value.type()
        // Use this_item as the value to get type of
        if let Some(this_item) = &context.this_item {
            match this_item {
                FhirPathValue::Collection(items) if items.len() == 1 => items[0].clone(),
                FhirPathValue::Collection(_) => {
                    return Err(FhirPathError::EvaluationError(
                        "'type' function cannot be applied to collections with multiple items".to_string(),
                    ));
                }
                other => other.clone(),
            }
        } else {
            return Err(FhirPathError::EvaluationError(
                "'type' function expects 1 argument or method call syntax".to_string(),
            ));
        }
    } else if arguments.len() == 1 {
        // Function call syntax: type(value)
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'type' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    let (namespace, name) = match result {
        FhirPathValue::Boolean(_) => ("System", "Boolean"),
        FhirPathValue::Integer(_) => ("System", "Integer"),
        FhirPathValue::Decimal(_) => ("System", "Decimal"),
        FhirPathValue::String(_) => ("System", "String"),
        FhirPathValue::Date(_) => ("System", "Date"),
        FhirPathValue::DateTime(_) => ("System", "DateTime"),
        FhirPathValue::Time(_) => ("System", "Time"),
        FhirPathValue::Quantity { .. } => ("System", "Quantity"),
        FhirPathValue::Collection(_) => ("System", "Collection"),
        FhirPathValue::Empty => return Ok(FhirPathValue::Empty),
        FhirPathValue::Resource(ref resource) => {
            if let Some(resource_type) = &resource.resource_type {
                ("FHIR", resource_type.as_str())
            } else {
                ("FHIR", "Resource")
            }
        }
    };

    // Create a type object with namespace and name properties
    let mut type_properties = std::collections::HashMap::new();
    type_properties.insert("namespace".to_string(), serde_json::Value::String(namespace.to_string()));
    type_properties.insert("name".to_string(), serde_json::Value::String(name.to_string()));

    let type_resource = FhirResource {
        resource_type: None,
        properties: type_properties,
    };

    Ok(FhirPathValue::Resource(type_resource))
}

fn evaluate_extension_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'extension' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    let url_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

    let url = match url_result {
        FhirPathValue::String(s) => s,
        _ => {
            return Err(FhirPathError::TypeError(
                "'extension' function requires a string URL argument".to_string(),
            ))
        }
    };

    // Get the current resource/object from context
    match &context.context {
        serde_json::Value::Object(obj) => {
            if let Some(extensions) = obj.get("extension") {
                if let serde_json::Value::Array(ext_array) = extensions {
                    let mut matching_extensions = Vec::new();

                    for ext in ext_array {
                        if let serde_json::Value::Object(ext_obj) = ext {
                            if let Some(ext_url) = ext_obj.get("url") {
                                if let serde_json::Value::String(ext_url_str) = ext_url {
                                    if ext_url_str == &url {
                                        matching_extensions
                                            .push(json_to_fhirpath_value(ext.clone())?);
                                    }
                                }
                            }
                        }
                    }

                    if matching_extensions.is_empty() {
                        Ok(FhirPathValue::Empty)
                    } else if matching_extensions.len() == 1 {
                        Ok(matching_extensions.into_iter().next().unwrap())
                    } else {
                        Ok(FhirPathValue::Collection(matching_extensions))
                    }
                } else {
                    Ok(FhirPathValue::Empty)
                }
            } else {
                Ok(FhirPathValue::Empty)
            }
        }
        _ => Ok(FhirPathValue::Empty),
    }
}

fn evaluate_of_type_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'ofType' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    let type_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

    let target_type = match type_result {
        FhirPathValue::String(s) => s,
        _ => {
            return Err(FhirPathError::TypeError(
                "'ofType' function requires a string type argument".to_string(),
            ))
        }
    };

    // Get the current collection from context
    let collection = get_current_collection(context)?;
    let mut filtered_results = Vec::new();

    for item in collection {
        let item_type = match &item {
            FhirPathValue::Boolean(_) => "System.Boolean",
            FhirPathValue::Integer(_) => "System.Integer",
            FhirPathValue::Decimal(_) => "System.Decimal",
            FhirPathValue::String(_) => "System.String",
            FhirPathValue::Date(_) => "System.Date",
            FhirPathValue::DateTime(_) => "System.DateTime",
            FhirPathValue::Time(_) => "System.Time",
            FhirPathValue::Quantity { .. } => "System.Quantity",
            FhirPathValue::Collection(_) => "System.Collection",
            FhirPathValue::Empty => continue,
            FhirPathValue::Resource(_) => "FHIR.Resource",
        };

        if item_type == target_type {
            filtered_results.push(item);
        }
    }

    if filtered_results.is_empty() {
        Ok(FhirPathValue::Empty)
    } else {
        Ok(FhirPathValue::Collection(filtered_results))
    }
}

fn evaluate_conforms_to_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'conformsTo' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    let _profile_result = evaluate_ast_with_visitor(&arguments[0], context, visitor)?;

    // For now, return a simple implementation that always returns true
    // In a full implementation, this would check if the resource conforms to the given profile
    Ok(FhirPathValue::Boolean(true))
}

fn evaluate_now_function(
    arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'now' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    // Return current datetime in ISO 8601 format
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| FhirPathError::EvaluationError(format!("System time error: {}", e)))?;

    // Convert to a basic ISO 8601 datetime string
    // This is a simplified implementation - in production you'd want proper datetime handling
    let secs = now.as_secs();
    let days_since_epoch = secs / 86400;
    let days_since_1970 = days_since_epoch;

    // Approximate calculation for current date/time
    // This is simplified - proper implementation would use chrono or similar
    let year = 1970 + (days_since_1970 / 365);
    let remaining_days = days_since_1970 % 365;
    let month = (remaining_days / 30) + 1;
    let day = (remaining_days % 30) + 1;

    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    let datetime_str = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month.min(12),
        day.min(31),
        hours,
        minutes,
        seconds
    );

    Ok(FhirPathValue::DateTime(datetime_str))
}

fn evaluate_today_function(
    arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(format!(
            "'today' function expects 0 arguments, got {}",
            arguments.len()
        )));
    }

    // Return current date in ISO 8601 format
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| FhirPathError::EvaluationError(format!("System time error: {}", e)))?;

    // Convert to a basic ISO 8601 date string
    let secs = now.as_secs();
    let days_since_epoch = secs / 86400;
    let days_since_1970 = days_since_epoch;

    // Approximate calculation for current date
    let year = 1970 + (days_since_1970 / 365);
    let remaining_days = days_since_1970 % 365;
    let month = (remaining_days / 30) + 1;
    let day = (remaining_days % 30) + 1;

    let date_str = format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(31));

    Ok(FhirPathValue::Date(date_str))
}

fn evaluate_time_of_day_function(
    _arguments: &[AstNode],
    _context: &EvaluationContext,
) -> Result<FhirPathValue, FhirPathError> {
    Err(FhirPathError::NotImplemented(
        "'timeOfDay' function not yet implemented".to_string(),
    ))
}

/// Evaluates the not() function
fn evaluate_not_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let result = if arguments.is_empty() {
        // Method call syntax: value.not()
        // Use this_item as the value to negate
        if let Some(this_item) = &context.this_item {
            match this_item {
                FhirPathValue::Collection(items) if items.len() == 1 => items[0].clone(),
                FhirPathValue::Collection(_) => {
                    return Err(FhirPathError::EvaluationError(
                        "'not' function cannot be applied to collections with multiple items".to_string(),
                    ));
                }
                other => other.clone(),
            }
        } else {
            return Err(FhirPathError::EvaluationError(
                "'not' function expects 1 argument or method call syntax".to_string(),
            ));
        }
    } else if arguments.len() == 1 {
        // Function call syntax: not(value)
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'not' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    match result {
        FhirPathValue::Boolean(b) => Ok(FhirPathValue::Boolean(!b)),
        FhirPathValue::Empty => Ok(FhirPathValue::Boolean(true)),
        FhirPathValue::Collection(ref items) if items.is_empty() => {
            Ok(FhirPathValue::Boolean(true))
        }
        _ => Ok(FhirPathValue::Boolean(false)),
    }
}

/// Evaluates the all() function - returns true if all items in the collection satisfy the given condition
fn evaluate_all_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() != 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'all' function expects 1 argument, got {}",
            arguments.len()
        )));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;
    let total = collection.len();

    // If collection is empty, all() returns true (vacuous truth)
    if collection.is_empty() {
        return Ok(FhirPathValue::Boolean(true));
    }

    // Evaluate the condition for each item in the collection
    for (idx, item) in collection.into_iter().enumerate() {
        // Create iteration context for this item
        let mut iteration_context = context.create_iteration_context(item, idx, total)?;

        // Evaluate the condition expression
        let condition_result = evaluate_ast_with_visitor(&arguments[0], &iteration_context, visitor)?;

        // Check if the condition is truthy
        if !is_truthy(&condition_result) {
            return Ok(FhirPathValue::Boolean(false));
        }
    }

    Ok(FhirPathValue::Boolean(true))
}

/// Evaluates the allTrue() function
fn evaluate_all_true_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    _visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(
            "'allTrue' function expects no arguments".to_string(),
        ));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    for item in collection {
        match item {
            FhirPathValue::Boolean(false) => return Ok(FhirPathValue::Boolean(false)),
            FhirPathValue::Boolean(true) => continue,
            FhirPathValue::Empty => continue, // Empty values are ignored
            _ => return Ok(FhirPathValue::Boolean(false)), // Non-boolean values make it false
        }
    }

    Ok(FhirPathValue::Boolean(true))
}

/// Evaluates the anyTrue() function
fn evaluate_any_true_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    _visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(
            "'anyTrue' function expects no arguments".to_string(),
        ));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    for item in collection {
        match item {
            FhirPathValue::Boolean(true) => return Ok(FhirPathValue::Boolean(true)),
            FhirPathValue::Boolean(false) => continue,
            FhirPathValue::Empty => continue, // Empty values are ignored
            _ => continue, // Non-boolean values are ignored for anyTrue
        }
    }

    Ok(FhirPathValue::Boolean(false))
}

/// Evaluates the allFalse() function
fn evaluate_all_false_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    _visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(
            "'allFalse' function expects no arguments".to_string(),
        ));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    for item in collection {
        match item {
            FhirPathValue::Boolean(true) => return Ok(FhirPathValue::Boolean(false)),
            FhirPathValue::Boolean(false) => continue,
            FhirPathValue::Empty => continue, // Empty values are ignored
            _ => return Ok(FhirPathValue::Boolean(false)), // Non-boolean values make it false
        }
    }

    Ok(FhirPathValue::Boolean(true))
}

/// Evaluates the anyFalse() function
fn evaluate_any_false_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    _visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if !arguments.is_empty() {
        return Err(FhirPathError::EvaluationError(
            "'anyFalse' function expects no arguments".to_string(),
        ));
    }

    // Get the current collection from context
    let collection = get_current_collection(context)?;

    for item in collection {
        match item {
            FhirPathValue::Boolean(false) => return Ok(FhirPathValue::Boolean(true)),
            FhirPathValue::Boolean(true) => continue,
            FhirPathValue::Empty => continue, // Empty values are ignored
            _ => continue, // Non-boolean values are ignored for anyFalse
        }
    }

    Ok(FhirPathValue::Boolean(false))
}

/// Evaluates the convertsToInteger() function
fn evaluate_converts_to_integer_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let result = if arguments.is_empty() {
        // Use current collection when no arguments provided
        let current_collection = get_current_collection(context)?;
        if current_collection.len() == 1 {
            current_collection[0].clone()
        } else if current_collection.is_empty() {
            FhirPathValue::Empty
        } else {
            FhirPathValue::Collection(current_collection)
        }
    } else if arguments.len() == 1 {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToInteger' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    let can_convert = match result {
        FhirPathValue::Integer(_) => true,
        FhirPathValue::Decimal(d) => d.fract() == 0.0, // Whole number decimals can be converted to integer
        FhirPathValue::String(s) => s.parse::<i64>().is_ok(),
        FhirPathValue::Boolean(_) => true,
        _ => false,
    };

    Ok(FhirPathValue::Boolean(can_convert))
}

/// Evaluates the convertsToString() function
fn evaluate_converts_to_string_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let _result = if arguments.is_empty() {
        // Use current context value when no arguments provided
        json_to_fhirpath_value(context.context.clone())?
    } else if arguments.len() == 1 {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToString' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    // Most values can be converted to string
    Ok(FhirPathValue::Boolean(true))
}

/// Evaluates the convertsToBoolean() function
fn evaluate_converts_to_boolean_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() > 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToBoolean' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    }

    let result = if arguments.is_empty() {
        // Use current collection when no arguments provided
        let current_collection = get_current_collection(context)?;
        if current_collection.len() == 1 {
            current_collection[0].clone()
        } else if current_collection.is_empty() {
            FhirPathValue::Empty
        } else {
            FhirPathValue::Collection(current_collection)
        }
    } else {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    };

    let can_convert = match result {
        FhirPathValue::Boolean(_) => true,
        FhirPathValue::String(s) => s == "true" || s == "false",
        FhirPathValue::Integer(i) => i == 0 || i == 1,
        FhirPathValue::Collection(ref items) => {
            items.len() == 1 && match &items[0] {
                FhirPathValue::Boolean(_) => true,
                FhirPathValue::String(s) => s == "true" || s == "false",
                FhirPathValue::Integer(i) => *i == 0 || *i == 1,
                _ => false,
            }
        }
        _ => false,
    };

    Ok(FhirPathValue::Boolean(can_convert))
}

/// Evaluates the convertsToDecimal() function
fn evaluate_converts_to_decimal_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    if arguments.len() > 1 {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToDecimal' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    }

    let result = if arguments.is_empty() {
        // Use current collection when no arguments provided
        let current_collection = get_current_collection(context)?;
        if current_collection.len() == 1 {
            current_collection[0].clone()
        } else if current_collection.is_empty() {
            FhirPathValue::Empty
        } else {
            FhirPathValue::Collection(current_collection)
        }
    } else {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    };

    let can_convert = match result {
        FhirPathValue::Decimal(_) => true,
        FhirPathValue::Integer(_) => true,
        FhirPathValue::String(s) => s.parse::<f64>().is_ok(),
        FhirPathValue::Collection(ref items) => {
            items.len() == 1 && match &items[0] {
                FhirPathValue::Decimal(_) => true,
                FhirPathValue::Integer(_) => true,
                FhirPathValue::String(s) => s.parse::<f64>().is_ok(),
                _ => false,
            }
        }
        _ => false,
    };

    Ok(FhirPathValue::Boolean(can_convert))
}

/// Evaluates the convertsToDate() function
fn evaluate_converts_to_date_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let result = if arguments.is_empty() {
        // Use current context value when no arguments provided
        json_to_fhirpath_value(context.context.clone())?
    } else if arguments.len() == 1 {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToDate' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    let can_convert = match result {
        FhirPathValue::Date(_) => true,
        FhirPathValue::DateTime(_) => true,
        FhirPathValue::String(s) => {
            // Basic date format validation
            s.len() >= 4 && s.chars().take(4).all(|c| c.is_ascii_digit())
        }
        _ => false,
    };

    Ok(FhirPathValue::Boolean(can_convert))
}

/// Evaluates the convertsToDateTime() function
fn evaluate_converts_to_date_time_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let result = if arguments.is_empty() {
        // Use current collection when no arguments provided
        let current_collection = get_current_collection(context)?;
        if current_collection.len() == 1 {
            current_collection[0].clone()
        } else if current_collection.is_empty() {
            FhirPathValue::Empty
        } else {
            FhirPathValue::Collection(current_collection)
        }
    } else if arguments.len() == 1 {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToDateTime' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    let can_convert = match result {
        FhirPathValue::DateTime(_) => true,
        FhirPathValue::Date(_) => true,
        FhirPathValue::String(s) => is_valid_datetime_string(&s),
        _ => false
    };

    Ok(FhirPathValue::Boolean(can_convert))
}

/// Converts a string to a DateTime value if possible
pub fn string_to_datetime(s: &str) -> Option<FhirPathValue> {
    if !is_valid_datetime_string(s) {
        return None;
    }

    // Handle time-only formats (starting with T)
    if s.starts_with('T') {
        return Some(FhirPathValue::Time(s.to_string()));
    }

    // Check if it contains 'T' to determine if it's a DateTime or Date
    if s.contains('T') {
        Some(FhirPathValue::DateTime(s.to_string()))
    } else {
        Some(FhirPathValue::Date(s.to_string()))
    }
}

/// Evaluates the convertsToQuantity() function
fn evaluate_converts_to_quantity_function(
    arguments: &[AstNode],
    context: &EvaluationContext,
    visitor: &dyn AstVisitor,
) -> Result<FhirPathValue, FhirPathError> {
    let result = if arguments.is_empty() {
        // Use current context value when no arguments provided
        json_to_fhirpath_value(context.context.clone())?
    } else if arguments.len() == 1 {
        evaluate_ast_with_visitor(&arguments[0], context, visitor)?
    } else {
        return Err(FhirPathError::EvaluationError(format!(
            "'convertsToQuantity' function expects 0 or 1 argument, got {}",
            arguments.len()
        )));
    };

    let can_convert = match result {
        FhirPathValue::Quantity { .. } => true,
        FhirPathValue::Integer(_) => true,
        FhirPathValue::Decimal(_) => true,
        FhirPathValue::String(s) => {
            // Basic quantity format validation (number followed by optional unit)
            s.split_whitespace()
                .next()
                .map_or(false, |part| part.parse::<f64>().is_ok())
        }
        _ => false,
    };

    Ok(FhirPathValue::Boolean(can_convert))
}

/// Helper function to get the current collection from context
fn get_current_collection(
    context: &EvaluationContext,
) -> Result<Vec<FhirPathValue>, FhirPathError> {
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
                }
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

/// Helper function to validate datetime string formats
fn is_valid_datetime_string(s: &str) -> bool {
    // Valid datetime formats according to FhirPath specification:
    // YYYY
    // YYYY-MM
    // YYYY-MM-DD
    // YYYY-MM-DDTHH
    // YYYY-MM-DDTHH:MM
    // YYYY-MM-DDTHH:MM:SS
    // YYYY-MM-DDTHH:MM:SS.SSS
    // With optional timezone: Z or +/-HH:MM
    // Time-only formats:
    // THH
    // THH:MM
    // THH:MM:SS
    // THH:MM:SS.SSS

    if s.is_empty() {
        return false;
    }

    // Handle time-only formats (starting with T)
    if s.starts_with('T') {
        return is_valid_time_string(&s[1..]);
    }

    // Check for year (YYYY)
    if s.len() >= 4 {
        let year_part = &s[0..4];
        if !year_part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // If it's just a year, it's valid
        if s.len() == 4 {
            return true;
        }

        // Check for month (YYYY-MM)
        if s.len() >= 7 && s.chars().nth(4) == Some('-') {
            let month_part = &s[5..7];
            if !month_part.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let month: u32 = month_part.parse().unwrap_or(0);
            if month < 1 || month > 12 {
                return false;
            }

            // If it's just year-month, it's valid
            if s.len() == 7 {
                return true;
            }

            // Check for day (YYYY-MM-DD)
            if s.len() >= 10 && s.chars().nth(7) == Some('-') {
                let day_part = &s[8..10];
                if !day_part.chars().all(|c| c.is_ascii_digit()) {
                    return false;
                }
                let day: u32 = day_part.parse().unwrap_or(0);
                if day < 1 || day > 31 {
                    return false;
                }

                // If it's just year-month-day, it's valid
                if s.len() == 10 {
                    return true;
                }

                // Check for time part (T)
                if s.len() >= 11 && s.chars().nth(10) == Some('T') {
                    // Validate the time part
                    return is_valid_time_string(&s[11..]);
                }
            }
        }
    }

    false
}

/// Helper function to validate time string formats
fn is_valid_time_string(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Check for hours (HH)
    if s.len() >= 2 {
        let hours_part = &s[0..2];
        if !hours_part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        let hours: u32 = hours_part.parse().unwrap_or(24);
        if hours > 23 {
            return false;
        }

        // If it's just hours, it's valid
        if s.len() == 2 {
            return true;
        }

        // Check for minutes (HH:MM)
        if s.len() >= 5 && s.chars().nth(2) == Some(':') {
            let minutes_part = &s[3..5];
            if !minutes_part.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let minutes: u32 = minutes_part.parse().unwrap_or(60);
            if minutes > 59 {
                return false;
            }

            // If it's just hours:minutes, it's valid
            if s.len() == 5 {
                return true;
            }

            // Check for seconds (HH:MM:SS)
            if s.len() >= 8 && s.chars().nth(5) == Some(':') {
                let seconds_part = &s[6..8];
                if !seconds_part.chars().all(|c| c.is_ascii_digit()) {
                    return false;
                }
                let seconds: u32 = seconds_part.parse().unwrap_or(60);
                if seconds > 59 {
                    return false;
                }

                // If it's just hours:minutes:seconds, it's valid
                if s.len() == 8 {
                    return true;
                }

                // Check for milliseconds (HH:MM:SS.SSS)
                if s.len() > 9 && s.chars().nth(8) == Some('.') {
                    let ms_part = &s[9..];
                    // Check if all remaining characters are digits (before timezone)
                    let ms_end = ms_part.find(|c| c == 'Z' || c == '+' || c == '-').unwrap_or(ms_part.len());
                    if !ms_part[..ms_end].chars().all(|c| c.is_ascii_digit()) {
                        return false;
                    }

                    // Check for timezone
                    if ms_end < ms_part.len() {
                        return is_valid_timezone(&ms_part[ms_end..]);
                    }
                    return true;
                }

                // Check for timezone after seconds
                if s.len() > 8 {
                    return is_valid_timezone(&s[8..]);
                }
            }
        }
    }

    false
}

/// Helper function to validate timezone formats
fn is_valid_timezone(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Z timezone
    if s == "Z" {
        return true;
    }

    // +/-HH:MM timezone
    if s.len() >= 3 && (s.starts_with('+') || s.starts_with('-')) {
        // Check hours part
        if s.len() >= 3 {
            let hours_part = &s[1..3];
            if !hours_part.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            let hours: u32 = hours_part.parse().unwrap_or(24);
            if hours > 23 {
                return false;
            }

            // If it's just +/-HH, it's valid
            if s.len() == 3 {
                return true;
            }

            // Check for colon
            if s.len() >= 4 && s.chars().nth(3) != Some(':') {
                return false;
            }

            // Check minutes part
            if s.len() >= 6 {
                let minutes_part = &s[4..6];
                if !minutes_part.chars().all(|c| c.is_ascii_digit()) {
                    return false;
                }
                let minutes: u32 = minutes_part.parse().unwrap_or(60);
                if minutes > 59 {
                    return false;
                }

                // If it's +/-HH:MM, it's valid
                if s.len() == 6 {
                    return true;
                }
            }
        }
    }

    false
}

/// Helper function to compare DateTime values with precision and timezone handling
fn datetime_equal(a: &str, b: &str) -> bool {
    // Handle different precision levels and timezone rules
    // Examples: @2012-04 should equal @2012-04-01T00:00:00Z when considering precision

    // Remove @ prefix if present
    let a_clean = a.strip_prefix('@').unwrap_or(a);
    let b_clean = b.strip_prefix('@').unwrap_or(b);

    // If both are exactly the same, they're equal
    if a_clean == b_clean {
        return true;
    }

    // Determine if we're comparing time-only values
    let a_is_time = a_clean.starts_with('T');
    let b_is_time = b_clean.starts_with('T');

    // If one is time-only and the other is not, they can't be equal
    if a_is_time != b_is_time {
        return false;
    }

    // Handle time-only comparison
    if a_is_time && b_is_time {
        return normalize_time(&a_clean[1..]) == normalize_time(&b_clean[1..]);
    }

    // Handle date/datetime comparison
    let normalized_a = normalize_datetime(a_clean);
    let normalized_b = normalize_datetime(b_clean);

    normalized_a == normalized_b
}

/// Helper function to normalize datetime strings for comparison
fn normalize_datetime(dt: &str) -> String {
    let mut normalized = dt.to_string();

    // Handle year-only precision: 2012 -> 2012-01-01T00:00:00
    if normalized.len() == 4 && normalized.chars().all(|c| c.is_ascii_digit()) {
        normalized = format!("{}-01-01T00:00:00", normalized);
    }
    // Handle year-month precision: 2012-04 -> 2012-04-01T00:00:00
    else if normalized.len() == 7 && normalized.matches('-').count() == 1 {
        normalized = format!("{}-01T00:00:00", normalized);
    }
    // Handle date precision: 2012-04-15 -> 2012-04-15T00:00:00
    else if normalized.len() == 10
        && normalized.matches('-').count() == 2
        && !normalized.contains('T')
    {
        normalized = format!("{}T00:00:00", normalized);
    }
    // Handle partial time formats
    else if normalized.contains('T') {
        let parts: Vec<&str> = normalized.split('T').collect();
        if parts.len() == 2 {
            let date_part = parts[0];
            let time_part = parts[1];

            // Normalize the time part
            let normalized_time = normalize_time(time_part);
            normalized = format!("{}T{}", date_part, normalized_time);
        }
    }

    // Normalize timezone to UTC (Z)
    // This is a simplified approach - a full implementation would convert between timezones
    if normalized.contains('+') || normalized.contains('-') || normalized.ends_with('Z') {
        // For now, just remove timezone info for comparison
        // A more accurate implementation would convert to a common timezone
        if let Some(tz_pos) = normalized.find('+') {
            normalized = normalized[..tz_pos].to_string();
        } else if let Some(tz_pos) = normalized.rfind('-') {
            if tz_pos > 10 {
                // Only remove timezone, not date separators
                normalized = normalized[..tz_pos].to_string();
            }
        } else if normalized.ends_with('Z') {
            normalized = normalized[..normalized.len() - 1].to_string();
        }
    }

    normalized
}

/// Helper function to normalize time strings for comparison
fn normalize_time(time: &str) -> String {
    let mut normalized = time.to_string();

    // Handle hours-only: HH -> HH:00:00
    if normalized.len() == 2 && normalized.chars().all(|c| c.is_ascii_digit()) {
        normalized = format!("{}:00:00", normalized);
    }
    // Handle hours-minutes: HH:MM -> HH:MM:00
    else if normalized.len() == 5 && normalized.chars().nth(2) == Some(':') {
        normalized = format!("{}:00", normalized);
    }

    // Handle timezone
    if normalized.contains('+') || normalized.contains('-') || normalized.ends_with('Z') {
        // For now, just remove timezone info for comparison
        if let Some(tz_pos) = normalized.find('+') {
            normalized = normalized[..tz_pos].to_string();
        } else if let Some(tz_pos) = normalized.find('-') {
            normalized = normalized[..tz_pos].to_string();
        } else if normalized.ends_with('Z') {
            normalized = normalized[..normalized.len() - 1].to_string();
        }
    }

    // Handle milliseconds
    if let Some(ms_pos) = normalized.find('.') {
        normalized = normalized[..ms_pos].to_string();
    }

    normalized
}

/// Generates an efficient cache key for an AST node using hashing
fn generate_cache_key(node: &AstNode) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_ast_node(node, &mut hasher);
    hasher.finish()
}

/// Determines if a node should be cached based on its complexity and potential for reuse
fn should_cache_node(node: &AstNode) -> bool {
    match node {
        // Don't cache simple literals - they're fast to evaluate
        AstNode::Identifier(_)
        | AstNode::StringLiteral(_)
        | AstNode::NumberLiteral(_)
        | AstNode::BooleanLiteral(_)
        | AstNode::DateTimeLiteral(_)
        | AstNode::Variable(_) => false,

        // Cache complex path expressions that might be expensive
        AstNode::Path(_, _) => true,

        // Cache function calls as they can be expensive
        AstNode::FunctionCall { .. } => true,

        // Cache complex binary operations but not simple ones
        AstNode::BinaryOp { op, left, right } => {
            match op {
                // Don't cache simple arithmetic/comparison on literals
                BinaryOperator::Addition
                | BinaryOperator::Subtraction
                | BinaryOperator::Multiplication
                | BinaryOperator::Division
                | BinaryOperator::Mod
                | BinaryOperator::Equals
                | BinaryOperator::NotEquals
                | BinaryOperator::LessThan
                | BinaryOperator::LessOrEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterOrEqual => {
                    // Only cache if operands are complex
                    !is_simple_node(left) || !is_simple_node(right)
                }
                // Cache logical operations as they might involve complex subexpressions
                BinaryOperator::And
                | BinaryOperator::Or
                | BinaryOperator::Xor
                | BinaryOperator::Implies
                | BinaryOperator::In
                | BinaryOperator::Union
                | BinaryOperator::Concatenation => true,
            }
        }

        // Don't cache simple unary operations
        AstNode::UnaryOp { operand, .. } => !is_simple_node(operand),

        // Cache indexing operations as they can be expensive
        AstNode::Indexer { .. } => true,
    }
}

/// Helper function to determine if a node is simple (fast to evaluate)
fn is_simple_node(node: &AstNode) -> bool {
    matches!(
        node,
        AstNode::Identifier(_)
            | AstNode::StringLiteral(_)
            | AstNode::NumberLiteral(_)
            | AstNode::BooleanLiteral(_)
            | AstNode::DateTimeLiteral(_)
    )
}

/// Recursively hashes an AST node structure
fn hash_ast_node(node: &AstNode, hasher: &mut DefaultHasher) {
    match node {
        AstNode::Identifier(name) => {
            0u8.hash(hasher);
            name.hash(hasher);
        }
        AstNode::StringLiteral(value) => {
            1u8.hash(hasher);
            value.hash(hasher);
        }
        AstNode::NumberLiteral(value) => {
            2u8.hash(hasher);
            value.to_bits().hash(hasher);
        }
        AstNode::BooleanLiteral(value) => {
            3u8.hash(hasher);
            value.hash(hasher);
        }
        AstNode::DateTimeLiteral(value) => {
            9u8.hash(hasher);
            value.hash(hasher);
        }
        AstNode::Variable(name) => {
            4u8.hash(hasher);
            name.hash(hasher);
        }
        AstNode::Path(left, right) => {
            4u8.hash(hasher);
            hash_ast_node(left, hasher);
            hash_ast_node(right, hasher);
        }
        AstNode::FunctionCall { name, arguments } => {
            5u8.hash(hasher);
            name.hash(hasher);
            arguments.len().hash(hasher);
            for arg in arguments {
                hash_ast_node(arg, hasher);
            }
        }
        AstNode::BinaryOp { op, left, right } => {
            6u8.hash(hasher);
            std::mem::discriminant(op).hash(hasher);
            hash_ast_node(left, hasher);
            hash_ast_node(right, hasher);
        }
        AstNode::UnaryOp { op, operand } => {
            7u8.hash(hasher);
            std::mem::discriminant(op).hash(hasher);
            hash_ast_node(operand, hasher);
        }
        AstNode::Indexer { collection, index } => {
            8u8.hash(hasher);
            hash_ast_node(collection, hasher);
            hash_ast_node(index, hasher);
        }
    }
}

/// Helper function to check if two values are equal
fn values_equal(left: &FhirPathValue, right: &FhirPathValue) -> bool {
    match (left, right) {
        (FhirPathValue::Empty, FhirPathValue::Empty) => true,
        (FhirPathValue::Boolean(a), FhirPathValue::Boolean(b)) => a == b,
        (FhirPathValue::Integer(a), FhirPathValue::Integer(b)) => a == b,
        (FhirPathValue::Decimal(a), FhirPathValue::Decimal(b)) => (a - b).abs() < f64::EPSILON,
        (FhirPathValue::Integer(a), FhirPathValue::Decimal(b)) => {
            (*a as f64 - b).abs() < f64::EPSILON
        }
        (FhirPathValue::Decimal(a), FhirPathValue::Integer(b)) => {
            (a - *b as f64).abs() < f64::EPSILON
        }
        (FhirPathValue::String(a), FhirPathValue::String(b)) => a == b,
        (FhirPathValue::Date(a), FhirPathValue::Date(b)) => datetime_equal(a, b),
        (FhirPathValue::DateTime(a), FhirPathValue::DateTime(b)) => datetime_equal(a, b),
        (FhirPathValue::Time(a), FhirPathValue::Time(b)) => a == b,
        (
            FhirPathValue::Quantity {
                value: v1,
                unit: u1,
            },
            FhirPathValue::Quantity {
                value: v2,
                unit: u2,
            },
        ) => (v1 - v2).abs() < f64::EPSILON && u1 == u2,
        _ => false,
    }
}
