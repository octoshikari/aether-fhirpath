// FHIRPath Lexer
//
// This module implements the lexical analysis for FHIRPath expressions.

use crate::errors::FhirPathError;
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

/// Token types for FHIRPath expressions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    // Literals
    Identifier,
    StringLiteral,
    NumberLiteral,
    BooleanLiteral,

    // Operators
    Dot,            // .
    Equal,          // =
    NotEqual,       // !=
    LessThan,       // <
    LessOrEqual,    // <=
    GreaterThan,    // >
    GreaterOrEqual, // >=
    Plus,           // +
    Minus,          // -
    Multiply,       // *
    Divide,         // /

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Comma,        // ,
    Pipe,         // |
    Colon,        // :

    // Special characters
    Backtick,  // `
    Dollar,    // $
    At,        // @
    Backslash, // \
    Percent,   // %

    // Keywords
    And,     // and
    Or,      // or
    Xor,     // xor
    Implies, // implies
    In,      // in
    Mod,     // mod

    // End of input
    EOF,
}

/// A token in a FHIRPath expression
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub position: usize,
    pub line: usize,
    pub column: usize,
}

/// Source span information for error reporting
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// Lexer for FHIRPath expressions
#[allow(dead_code)]
pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    position: usize,
    line: usize,
    column: usize,
    keywords: HashMap<String, TokenType>,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        let mut keywords = HashMap::new();
        keywords.insert("and".to_string(), TokenType::And);
        keywords.insert("or".to_string(), TokenType::Or);
        keywords.insert("xor".to_string(), TokenType::Xor);
        keywords.insert("implies".to_string(), TokenType::Implies);
        keywords.insert("in".to_string(), TokenType::In);
        keywords.insert("mod".to_string(), TokenType::Mod);
        keywords.insert("true".to_string(), TokenType::BooleanLiteral);
        keywords.insert("false".to_string(), TokenType::BooleanLiteral);

        Lexer {
            input,
            chars: input.chars().peekable(),
            position: 0,
            line: 1,
            column: 1,
            keywords,
        }
    }

    /// Advances the lexer by one character
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            self.position += 1;
            self.column += 1;

            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            }
        }
        c
    }

    /// Peeks at the next character without advancing
    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Creates a token with the current position information
    fn make_token(&self, token_type: TokenType, lexeme: String) -> Token {
        let len = lexeme.len();
        Token {
            token_type,
            lexeme,
            position: self.position - len,
            line: self.line,
            column: self.column - len,
        }
    }

    /// Skips whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.advance();
        }
    }

    /// Tokenizes an identifier or keyword
    fn identifier(&mut self) -> Result<Token, FhirPathError> {
        let start_pos = self.position;
        let start_column = self.column;
        let start_line = self.line;

        let mut identifier = String::new();

        // First character is already checked to be valid by the caller
        if let Some(c) = self.advance() {
            identifier.push(c);
        }

        // Continue reading valid identifier characters
        while let Some(&c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                identifier.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Check if it's a keyword
        let token_type = if let Some(keyword_type) = self.keywords.get(&identifier) {
            *keyword_type
        } else {
            TokenType::Identifier
        };

        Ok(Token {
            token_type,
            lexeme: identifier,
            position: start_pos,
            line: start_line,
            column: start_column,
        })
    }

    /// Tokenizes a number literal
    fn number(&mut self) -> Result<Token, FhirPathError> {
        let start_pos = self.position;
        let start_column = self.column;
        let start_line = self.line;

        let mut number = String::new();
        let mut has_decimal = false;

        // Continue reading digits
        while let Some(&c) = self.peek() {
            if c.is_ascii_digit() {
                number.push(c);
                self.advance();
            } else if c == '.' && !has_decimal {
                // Check if there's a digit after the decimal point
                // Look ahead without consuming the dot
                let mut temp_pos = self.position + 1;
                if temp_pos < self.input.len() {
                    let next_char = self.input.chars().nth(temp_pos).unwrap();
                    if next_char.is_ascii_digit() {
                        // It's a decimal number, consume the dot and include it
                        self.advance(); // consume the dot
                        has_decimal = true;
                        number.push(c);
                        // Continue to read the digits after the decimal point
                    } else {
                        // It's not a decimal number (probably a method call like "1.round()")
                        // Don't consume the dot, let it be tokenized separately
                        break;
                    }
                } else {
                    // End of input after decimal point - not a valid decimal
                    return Err(FhirPathError::LexerError(format!(
                        "Expected digit after decimal point at line {}, column {}",
                        self.line, self.column + 1
                    )));
                }
            } else {
                break;
            }
        }

        Ok(Token {
            token_type: TokenType::NumberLiteral,
            lexeme: number,
            position: start_pos,
            line: start_line,
            column: start_column,
        })
    }

    /// Tokenizes a string literal
    fn string(&mut self) -> Result<Token, FhirPathError> {
        let start_pos = self.position;
        let start_column = self.column;
        let start_line = self.line;

        // Skip the opening quote
        self.advance();

        let mut string = String::new();

        // Read until closing quote
        while let Some(&c) = self.peek() {
            if c == '\'' {
                // Skip the closing quote
                self.advance();

                // Check for escaped quote
                if let Some(&next) = self.peek() {
                    if next == '\'' {
                        // It's an escaped quote, include it and continue
                        string.push('\'');
                        self.advance();
                        continue;
                    }
                }

                // It's the end of the string
                return Ok(Token {
                    token_type: TokenType::StringLiteral,
                    lexeme: string,
                    position: start_pos,
                    line: start_line,
                    column: start_column,
                });
            } else if c == '\n' {
                return Err(FhirPathError::LexerError(format!(
                    "Unterminated string literal at line {}",
                    start_line
                )));
            } else {
                string.push(c);
                self.advance();
            }
        }

        // If we get here, the string wasn't terminated
        Err(FhirPathError::LexerError(format!(
            "Unterminated string literal at line {}, column {}",
            start_line, start_column
        )))
    }

    /// Scans the next token
    pub fn scan_token(&mut self) -> Result<Token, FhirPathError> {
        self.skip_whitespace();

        if let Some(&c) = self.peek() {
            match c {
                // Single-character tokens
                '(' => {
                    self.advance();
                    Ok(self.make_token(TokenType::LeftParen, "(".to_string()))
                }
                ')' => {
                    self.advance();
                    Ok(self.make_token(TokenType::RightParen, ")".to_string()))
                }
                '[' => {
                    self.advance();
                    Ok(self.make_token(TokenType::LeftBracket, "[".to_string()))
                }
                ']' => {
                    self.advance();
                    Ok(self.make_token(TokenType::RightBracket, "]".to_string()))
                }
                '{' => {
                    self.advance();
                    Ok(self.make_token(TokenType::LeftBrace, "{".to_string()))
                }
                '}' => {
                    self.advance();
                    Ok(self.make_token(TokenType::RightBrace, "}".to_string()))
                }
                ',' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Comma, ",".to_string()))
                }
                '|' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Pipe, "|".to_string()))
                }
                ':' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Colon, ":".to_string()))
                }
                '.' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Dot, ".".to_string()))
                }
                '+' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Plus, "+".to_string()))
                }
                '-' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Minus, "-".to_string()))
                }
                '*' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Multiply, "*".to_string()))
                }
                '/' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Divide, "/".to_string()))
                }

                // Special characters
                '`' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Backtick, "`".to_string()))
                }
                '$' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Dollar, "$".to_string()))
                }
                '@' => {
                    self.advance();
                    Ok(self.make_token(TokenType::At, "@".to_string()))
                }
                '\\' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Backslash, "\\".to_string()))
                }
                '%' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Percent, "%".to_string()))
                }
                '&' => {
                    self.advance();
                    Ok(self.make_token(TokenType::And, "&".to_string()))
                }

                // Two-character tokens
                '=' => {
                    self.advance();
                    Ok(self.make_token(TokenType::Equal, "=".to_string()))
                }
                '!' => {
                    self.advance();
                    if let Some(&next) = self.peek() {
                        if next == '=' {
                            self.advance();
                            return Ok(self.make_token(TokenType::NotEqual, "!=".to_string()));
                        }
                    }
                    Err(FhirPathError::LexerError(format!(
                        "Unexpected character '!' at line {}, column {}",
                        self.line,
                        self.column - 1
                    )))
                }
                '<' => {
                    self.advance();
                    if let Some(&next) = self.peek() {
                        if next == '=' {
                            self.advance();
                            return Ok(self.make_token(TokenType::LessOrEqual, "<=".to_string()));
                        }
                    }
                    Ok(self.make_token(TokenType::LessThan, "<".to_string()))
                }
                '>' => {
                    self.advance();
                    if let Some(&next) = self.peek() {
                        if next == '=' {
                            self.advance();
                            return Ok(self.make_token(TokenType::GreaterOrEqual, ">=".to_string()));
                        }
                    }
                    Ok(self.make_token(TokenType::GreaterThan, ">".to_string()))
                }

                // String literals
                '\'' => self.string(),

                // Number literals
                '0'..='9' => self.number(),

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => self.identifier(),

                // Unexpected character
                _ => Err(FhirPathError::LexerError(format!(
                    "Unexpected character '{}' at line {}, column {}",
                    c, self.line, self.column
                ))),
            }
        } else {
            // End of input
            Ok(self.make_token(TokenType::EOF, "".to_string()))
        }
    }
}

/// Tokenizes a FHIRPath expression
pub fn tokenize(input: &str) -> Result<Vec<Token>, FhirPathError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.scan_token()?;
        let is_eof = token.token_type == TokenType::EOF;
        tokens.push(token);

        if is_eof {
            break;
        }
    }

    Ok(tokens)
}
