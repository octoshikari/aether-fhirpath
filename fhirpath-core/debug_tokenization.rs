use fhirpath_core::lexer::tokenize;

fn main() {
    let test_expressions = vec![
        "@2015-02-04",
        "@2015-02-04T14",
        "@2015-02-04T14:30",
        "@2015-02-04T14:30:45",
    ];

    for expr in test_expressions {
        println!("\n=== Tokenizing: {} ===", expr);
        match tokenize(expr) {
            Ok(tokens) => {
                for (i, token) in tokens.iter().enumerate() {
                    println!("  {}: {:?}", i, token);
                }
            }
            Err(e) => {
                println!("  Error: {:?}", e);
            }
        }
    }
}
