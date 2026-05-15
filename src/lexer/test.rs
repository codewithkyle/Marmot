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

#[test]
fn lexes_double_quoted_string() {
    let mut lexer = Lexer::new("\"fonts/Helvetica.ttf\"");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(
        tokens[0].kind,
        TokenKind::String("fonts/Helvetica.ttf".to_string())
    );
}

#[test]
fn lexes_escaped_characters_in_double_quoted_string() {
    let mut lexer = Lexer::new("\"line\\ncol\\tend\"");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(
        tokens[0].kind,
        TokenKind::String("line\ncol\tend".to_string())
    );
}

#[test]
fn unknown_escape_char_is_passed_through() {
    let mut lexer = Lexer::new(r#""a\z""#);
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::String("az".to_string()));
}

#[test]
fn errors_on_number_with_trailing_dot() {
    let mut lexer = Lexer::new("12.");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::InvalidNumber {
            value: "12.".to_string(),
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn errors_on_number_with_multiple_dots() {
    let mut lexer = Lexer::new("1.2.3");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::InvalidNumber {
            value: "1.2.".to_string(),
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn errors_on_negative_number_literal() {
    let mut lexer = Lexer::new("-1");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::UnknownCharacter {
            ch: '-',
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn errors_on_dot_prefixed_number_literal() {
    let mut lexer = Lexer::new(".5");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::UnknownCharacter {
            ch: '.',
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn errors_on_scientific_notation_number_literal() {
    let mut lexer = Lexer::new("1e3");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::InvalidNumber {
            value: "1e".to_string(),
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn lexes_slot_with_digits_and_underscores() {
    let mut lexer = Lexer::new("$(product_1_name)");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(
        tokens[0].kind,
        TokenKind::Slot("product_1_name".to_string())
    );
}

#[test]
fn errors_on_slot_starting_with_digit() {
    let mut lexer = Lexer::new("$(1name)");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(err, LexError::InvalidSlotVariable { line: 1, column: 1 });
}

#[test]
fn errors_on_slot_containing_dash() {
    let mut lexer = Lexer::new("$(product-name)");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(err, LexError::InvalidSlotVariable { line: 1, column: 1 });
}

#[test]
fn errors_on_unterminated_slot_across_newline() {
    let mut lexer = Lexer::new("$(product\nname)");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::UnterminatedSlotVariable { line: 1, column: 1 }
    );
}

#[test]
fn reports_line_and_column_for_multiline_unknown_character() {
    let mut lexer = Lexer::new("page\n@");
    let err = lexer.tokenize().unwrap_err();

    assert_eq!(
        err,
        LexError::UnknownCharacter {
            ch: '@',
            line: 2,
            column: 1,
        }
    );
}

#[test]
fn trims_comment_whitespace() {
    let mut lexer = Lexer::new("%   hello   ");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Comment("hello".to_string()));
}
