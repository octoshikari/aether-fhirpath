// Debug test for lexer

use fhirpath_core::lexer::{TokenType, tokenize};

#[test]
#[allow(clippy::assertions_on_constants)]
fn debug_complex_expression() {
    let expr = "Patient.name[0].given[0] = 'John' and Patient.gender = 'male'";
    let tokens = tokenize(expr).unwrap();

    println!("Token count (excluding EOF): {}", tokens.len() - 1);

    for (i, token) in tokens.iter().enumerate() {
        if token.token_type == TokenType::EOF {
            println!("{}: EOF", i);
        } else {
            println!("{}: {:?} - '{}'", i, token.token_type, token.lexeme);
        }
    }

    // This will always pass, just for debugging

    assert!(true);
}
