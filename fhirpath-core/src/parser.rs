// FHIRPath Parser
//
// This module implements the parser for FHIRPath expressions.

use crate::errors::FhirPathError;
use crate::lexer::{Token, TokenType};

/// AST node types for FHIRPath expressions
#[derive(Debug, Clone)]
pub enum AstNode {
    // Literals
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),

    // Path navigation
    Path(Box<AstNode>, Box<AstNode>),

    // Function calls
    FunctionCall {
        name: String,
        arguments: Vec<AstNode>,
    },

    // Operators
    BinaryOp {
        op: BinaryOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },

    UnaryOp {
        op: UnaryOperator,
        operand: Box<AstNode>,
    },

    // Indexing
    Indexer {
        collection: Box<AstNode>,
        index: Box<AstNode>,
    },
}

/// Binary operators in FHIRPath
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Equals,
    NotEquals,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    Addition,
    Subtraction,
    Multiplication,
    Division,
    And,
    Or,
    Xor,
    Implies,
    In,
}

/// Unary operators in FHIRPath
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

/// Parser for FHIRPath expressions
pub struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl<'a> Parser<'a> {
    /// Creates a new parser
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            current: 0,
        }
    }

    /// Parses a FHIRPath expression
    pub fn parse(&mut self) -> Result<AstNode, FhirPathError> {
        self.expression()
    }

    /// Checks if we've reached the end of the token stream
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.peek().token_type == TokenType::EOF
    }

    /// Returns the current token without advancing
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// Returns the previous token
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    /// Advances to the next token and returns the current one
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    /// Checks if the current token matches the given type
    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().token_type == token_type
        }
    }

    /// Consumes the current token if it matches the given type
    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consumes the current token if it matches any of the given types
    fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        for token_type in token_types {
            if self.match_token(*token_type) {
                return true;
            }
        }
        false
    }

    /// Consumes the current token if it matches the given type, otherwise throws an error
    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token, FhirPathError> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(FhirPathError::ParserError(format!(
                "{} at token {:?}",
                message,
                self.peek()
            )))
        }
    }

    /// Parses an expression
    fn expression(&mut self) -> Result<AstNode, FhirPathError> {
        self.logical_implies()
    }

    /// Parses a logical IMPLIES expression
    fn logical_implies(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.logical_or()?;

        while self.match_token(TokenType::Implies) {
            let right = self.logical_or()?;
            expr = AstNode::BinaryOp {
                op: BinaryOperator::Implies,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses a logical OR expression
    fn logical_or(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.logical_and()?;

        while self.match_any(&[TokenType::Or, TokenType::Xor]) {
            let operator = match self.previous().token_type {
                TokenType::Or => BinaryOperator::Or,
                TokenType::Xor => BinaryOperator::Xor,
                _ => unreachable!(),
            };
            let right = self.logical_and()?;
            expr = AstNode::BinaryOp {
                op: operator,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses a logical AND expression
    fn logical_and(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.equality()?;

        while self.match_token(TokenType::And) {
            let right = self.equality()?;
            expr = AstNode::BinaryOp {
                op: BinaryOperator::And,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses an equality expression
    fn equality(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.comparison()?;

        while self.match_any(&[TokenType::Equal, TokenType::NotEqual]) {
            let operator = match self.previous().token_type {
                TokenType::Equal => BinaryOperator::Equals,
                TokenType::NotEqual => BinaryOperator::NotEquals,
                _ => unreachable!(),
            };
            let right = self.comparison()?;
            expr = AstNode::BinaryOp {
                op: operator,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses a comparison expression
    fn comparison(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.term()?;

        while self.match_any(&[
            TokenType::LessThan,
            TokenType::LessOrEqual,
            TokenType::GreaterThan,
            TokenType::GreaterOrEqual,
        ]) {
            let operator = match self.previous().token_type {
                TokenType::LessThan => BinaryOperator::LessThan,
                TokenType::LessOrEqual => BinaryOperator::LessOrEqual,
                TokenType::GreaterThan => BinaryOperator::GreaterThan,
                TokenType::GreaterOrEqual => BinaryOperator::GreaterOrEqual,
                _ => unreachable!(),
            };
            let right = self.term()?;
            expr = AstNode::BinaryOp {
                op: operator,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses a term expression (addition, subtraction)
    fn term(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.factor()?;

        while self.match_any(&[TokenType::Plus, TokenType::Minus]) {
            let operator = match self.previous().token_type {
                TokenType::Plus => BinaryOperator::Addition,
                TokenType::Minus => BinaryOperator::Subtraction,
                _ => unreachable!(),
            };
            let right = self.factor()?;
            expr = AstNode::BinaryOp {
                op: operator,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses a factor expression (multiplication, division)
    fn factor(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.unary()?;

        while self.match_any(&[TokenType::Multiply, TokenType::Divide]) {
            let operator = match self.previous().token_type {
                TokenType::Multiply => BinaryOperator::Multiplication,
                TokenType::Divide => BinaryOperator::Division,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            expr = AstNode::BinaryOp {
                op: operator,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parses a unary expression
    fn unary(&mut self) -> Result<AstNode, FhirPathError> {
        if self.match_token(TokenType::Minus) {
            let right = self.unary()?;
            Ok(AstNode::UnaryOp {
                op: UnaryOperator::Negate,
                operand: Box::new(right),
            })
        } else {
            self.path()
        }
    }

    /// Parses a path expression
    fn path(&mut self) -> Result<AstNode, FhirPathError> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(TokenType::Dot) {
                // Path navigation
                let right = self.primary()?;
                expr = AstNode::Path(Box::new(expr), Box::new(right));
            } else if self.match_token(TokenType::LeftBracket) {
                // Indexer
                let index = self.expression()?;
                self.consume(TokenType::RightBracket, "Expected ']' after index")?;
                expr = AstNode::Indexer {
                    collection: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parses a primary expression
    fn primary(&mut self) -> Result<AstNode, FhirPathError> {
        if self.match_token(TokenType::Identifier) {
            let name = self.previous().lexeme.clone();

            // Check if this is a function call
            if self.match_token(TokenType::LeftParen) {
                let mut arguments = Vec::new();

                // Parse arguments
                if !self.check(TokenType::RightParen) {
                    loop {
                        arguments.push(self.expression()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.consume(TokenType::RightParen, "Expected ')' after function arguments")?;

                Ok(AstNode::FunctionCall {
                    name,
                    arguments,
                })
            } else {
                Ok(AstNode::Identifier(name))
            }
        } else if self.match_token(TokenType::StringLiteral) {
            Ok(AstNode::StringLiteral(self.previous().lexeme.clone()))
        } else if self.match_token(TokenType::NumberLiteral) {
            let value = self.previous().lexeme.parse::<f64>().map_err(|e| {
                FhirPathError::ParserError(format!("Invalid number: {}", e))
            })?;
            Ok(AstNode::NumberLiteral(value))
        } else if self.match_token(TokenType::BooleanLiteral) {
            let value = match self.previous().lexeme.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(FhirPathError::ParserError("Invalid boolean literal".to_string())),
            };
            Ok(AstNode::BooleanLiteral(value))
        } else if self.match_token(TokenType::LeftParen) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expected ')' after expression")?;
            Ok(expr)
        } else {
            Err(FhirPathError::ParserError(format!(
                "Expected expression, got {:?}",
                self.peek()
            )))
        }
    }
}

/// Parses a FHIRPath expression from tokens
pub fn parse(tokens: &[Token]) -> Result<AstNode, FhirPathError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}
