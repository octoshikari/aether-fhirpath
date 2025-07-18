use fhirpath_core::lexer::tokenize;

fn main() {
    let expressions = vec![
        "1.convertsToInteger()",
        "1.0.convertsToInteger()",
        "'1'.convertsToInteger()",
    ];
    
    for expr in expressions {
        println!("\nTokenizing: {}", expr);
        match tokenize(expr) {
            Ok(tokens) => {
                for (i, token) in tokens.iter().enumerate() {
                    println!("  Token {}: {:?} - '{}'", i, token.token_type, token.lexeme);
                }
            }
            Err(e) => println!("  Error: {:?}", e),
        }
    }
}
