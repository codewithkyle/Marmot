use super::*;

#[test]
fn lexes_empty_file() {
    let mut lexer = Lexer::new("");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn errors_on_unknown_character() {
    let mut lexer = Lexer::new("@");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::UnknownCharacter {
            ch: '@',
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn lexes_comment() {
    let mut lexer = Lexer::new("% hello");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Comment("hello".to_string()));
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn lexes_comment_then_word_on_next_line() {
    let mut lexer = Lexer::new("% hello\npage");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Comment("hello".to_string()));
    assert_eq!(tokens[1].kind, TokenKind::Word("page".to_string()));
}

#[test]
fn lexes_string() {
    let mut lexer = Lexer::new("(Hello world)");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::String("Hello world".to_string()));
}

#[test]
fn lexes_escaped_parens() {
    let mut lexer = Lexer::new(r"(Hello \( world \))");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(
        tokens[0].kind,
        TokenKind::String("Hello ( world )".to_string())
    );
}

#[test]
fn errors_on_unterminated_string() {
    let mut lexer = Lexer::new("(Hello world");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(err, LexError::UnterminatedString { line: 1, column: 1 });
}

#[test]
fn lexes_slot_variable() {
    let mut lexer = Lexer::new("$(product_name)");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Slot("product_name".to_string()));
}

#[test]
fn errors_on_slot_without_open_paren() {
    let mut lexer = Lexer::new("$product_name)");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(err, LexError::InvalidSlotVariable { line: 1, column: 1 });
}

#[test]
fn errors_on_empty_slot() {
    let mut lexer = Lexer::new("$()");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(err, LexError::InvalidSlotVariable { line: 1, column: 1 });
}

#[test]
fn errors_on_unterminated_slot() {
    let mut lexer = Lexer::new("$(product_name");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::UnterminatedSlotVariable { line: 1, column: 1 }
    );
}

#[test]
fn lexes_words() {
    let mut lexer = Lexer::new("page draw begin product_name");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Word("page".to_string()));
    assert_eq!(tokens[1].kind, TokenKind::Word("draw".to_string()));
    assert_eq!(tokens[2].kind, TokenKind::Word("begin".to_string()));
    assert_eq!(tokens[3].kind, TokenKind::Word("product_name".to_string()));
}

#[test]
fn lexes_numbers() {
    let mut lexer = Lexer::new("612 72.5 0.25");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Number(612.0));
    assert_eq!(tokens[1].kind, TokenKind::Number(72.5));
    assert_eq!(tokens[2].kind, TokenKind::Number(0.25));
}

#[test]
fn errors_on_invalid_number_with_word_suffix() {
    let mut lexer = Lexer::new("72rect");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::InvalidNumber {
            value: "72r".to_string(),
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn errors_on_invalid_number_with_comma() {
    let mut lexer = Lexer::new("72,100");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::InvalidNumber {
            value: "72,".to_string(),
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn lexes_small_template() {
    let source = r#"
%!PSL 0.1

page 612 792

draw begin
  1 0 0 rgb
  $(product_name) 72 100 468 80 textbox
  (Hello \(world\)) 72 200 468 80 textbox
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Word("page".to_string()))
    );
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Number(612.0)));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Slot("product_name".to_string()))
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::String("Hello (world)".to_string()))
    );
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}
