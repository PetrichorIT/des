#[test]
fn test_parser() {
    use super::tokenize;

    let contents = std::fs::read_to_string("./examples/NetworkStack.ndl").expect("msg");

    let res = tokenize(&contents);

    let mut validated_token_stream = std::collections::VecDeque::new();

    for token in res {
        if !token.kind.valid() {
            // ERRs
            continue;
        }

        if !token.kind.reducable() {
            validated_token_stream.push_back(token)
        }
    }
}
