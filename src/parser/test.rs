use super::*;
use crate::lexer::Lexer;

#[test]
fn parses_header_and_page() {
    let source = r#"%!PSL 0.1
page 612 792
draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(template.version, "0.1");
    assert_eq!(
        template.page,
        Page {
            width: 612.0,
            height: 792.0,
        }
    );
}

#[test]
fn errors_without_header() {
    let source = r#"page 612 792"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(err, ParseError::ExpectedHeader { line: 1, column: 1 });
}

#[test]
fn errors_when_page_width_is_missing() {
    let source = r#"%!PSL 0.1
page
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::ExpectedNumber {
            found: TokenKind::Eof,
            line: 3,
            column: 1,
        }
    );
}

#[test]
fn errors_when_extra_tokens_exist() {
    let source = r#"%!PSL 0.1
page 612 792
draw begin
end
extra
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::ExpectedEof {
            found: TokenKind::Word("extra".to_string()),
            line: 5,
            column: 1,
        }
    );
}

#[test]
fn parses_template_without_slots() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(template.slots, Vec::new());
}

#[test]
fn parses_one_slot() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  product_name string required
end

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.slots,
        vec![SlotDecl {
            name: "product_name".to_string(),
            ty: SlotType::String,
            required: true,
        }]
    );
}

#[test]
fn parses_multiple_slots() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  product_name string required
  buy int required
  sale_price decimal required
end

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(template.slots.len(), 3);
    assert_eq!(template.slots[0].name, "product_name");
    assert_eq!(template.slots[1].ty, SlotType::Int);
    assert_eq!(template.slots[2].ty, SlotType::Decimal);
}

#[test]
fn errors_on_invalid_slot_type() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  product_name text required
end

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::InvalidSlotType {
            value: "text".to_string(),
        }
    );
}

#[test]
fn parse_optional_slot() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  product_desc string
  product_name string required
  price string required
end

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(template.slots.len(), 3);
    assert_eq!(template.slots[0].name, "product_desc");
    assert_eq!(template.slots[0].ty, SlotType::String);
    assert_eq!(template.slots[0].required, false);
}

#[test]
fn parses_simple_draw_block() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  1 0 0 rgb
  2 strokewidth
  0 0 10 10 line stroke
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![
            DrawOp::SetRgb {
                r: NumberValue::Literal(1.0),
                g: NumberValue::Literal(0.0),
                b: NumberValue::Literal(0.0),
            },
            DrawOp::SetStrokeWidth {
                width: NumberValue::Literal(2.0)
            },
            DrawOp::LinePath {
                x1: NumberValue::Literal(0.0),
                y1: NumberValue::Literal(0.0),
                x2: NumberValue::Literal(10.0),
                y2: NumberValue::Literal(10.0),
            },
            DrawOp::Stroke,
        ]
    );
}

#[test]
fn errors_when_rgb_has_too_few_values() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  1 0 rgb
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::StackUnderflow {
            operator: "rgb".to_string(),
            expected: 3,
            actual: 2,
            line: 5,
            column: 7,
        }
    );
}

#[test]
fn errors_when_line_has_too_few_values() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  0 0 10 line
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::StackUnderflow {
            operator: "line".to_string(),
            expected: 4,
            actual: 3,
            line: 5,
            column: 10,
        }
    );
}

#[test]
fn parses_static_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  (Hello world) 0 0 100 100 textbox
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![DrawOp::TextBox {
            text: TextValue::Literal("Hello world".to_string()),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(100.0),
        }]
    );
}

#[test]
fn errors_when_textbox_text_is_missing() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  0 0 100 100 textbox
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::StackUnderflow {
            operator: "textbox".to_string(),
            expected: 5,
            actual: 4,
            line: 5,
            column: 15,
        }
    );
}

#[test]
fn errors_when_textbox_text_is_not_string() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  1 0 0 100 100 textbox
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnexpectedStackValue {
            operator: "textbox".to_string(),
            expected: "string".to_string(),
            found: "number".to_string(),
            line: 5,
            column: 17,
        }
    );
}

#[test]
fn parses_numeric_slot_in_rect() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  x decimal
end

draw begin
  $(x) 0 10 10 rect stroke
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![
            DrawOp::RectPath {
                x: NumberValue::Slot("x".to_string()),
                y: NumberValue::Literal(0.0),
                width: NumberValue::Literal(10.0),
                height: NumberValue::Literal(10.0),
            },
            DrawOp::Stroke,
        ]
    );
}

#[test]
fn parses_rect_fill() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  0 0 10 10 rect fill
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![
            DrawOp::RectPath {
                x: NumberValue::Literal(0.0),
                y: NumberValue::Literal(0.0),
                width: NumberValue::Literal(10.0),
                height: NumberValue::Literal(10.0),
            },
            DrawOp::Fill,
        ]
    );
}

#[test]
fn errors_on_unknown_slot_in_draw() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  $(missing) 0 0 100 100 textbox
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnknownSlot {
            name: "missing".to_string(),
            line: 5,
            column: 3,
        }
    );
}

#[test]
fn parses_string_slot_in_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  product_name string
end

draw begin
  $(product_name) 0 0 100 100 textbox
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![DrawOp::TextBox {
            text: TextValue::Slot("product_name".to_string()),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(100.0),
        }]
    );
}

#[test]
fn errors_on_invalid_literal_rgb_component() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  2 0 0 rgb
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::InvalidNumberOperand {
            operator: "rgb".to_string(),
            operand: "r".to_string(),
            value: 2.0,
            expected: "0..=1".to_string(),
            line: 5,
            column: 9,
        }
    );
}

#[test]
fn allows_slot_rgb_component() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  r decimal
end

draw begin
  $(r) 0 0 rgb
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![DrawOp::SetRgb {
            r: NumberValue::Slot("r".to_string()),
            g: NumberValue::Literal(0.0),
            b: NumberValue::Literal(0.0),
        }]
    );
}

#[test]
fn errors_on_zero_rect_width() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  0 0 0 10 rect fill
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::InvalidNumberOperand {
            operator: "rect".to_string(),
            operand: "width".to_string(),
            value: 0.0,
            expected: "> 0".to_string(),
            line: 5,
            column: 12,
        }
    );
}

#[test]
fn allows_zero_line_coordinates() {
    let source = r#"%!PSL 0.1
page 612 792

draw begin
  0 0 10 10 line stroke
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw,
        vec![
            DrawOp::LinePath {
                x1: NumberValue::Literal(0.0),
                y1: NumberValue::Literal(0.0),
                x2: NumberValue::Literal(10.0),
                y2: NumberValue::Literal(10.0),
            },
            DrawOp::Stroke,
        ]
    );
}

#[test]
fn parses_fonts_block() {
    let source = r#"%!PSL 0.1
page 612 792

fonts begin
  helvetica "fonts/Helvetica.ttf"
  helvetica_bold "fonts/Helvetica-Bold.ttf"
end

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.fonts,
        vec![
            FontDecl {
                name: "helvetica".to_string(),
                path: "fonts/Helvetica.ttf".to_string(),
            },
            FontDecl {
                name: "helvetica_bold".to_string(),
                path: "fonts/Helvetica-Bold.ttf".to_string(),
            },
        ]
    );
}

#[test]
fn errors_when_font_path_is_missing() {
    let source = r#"%!PSL 0.1
page 612 792

fonts begin
  helvetica
end

draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::ExpectedString {
            found: TokenKind::Word("end".to_string()),
            line: 6,
            column: 1,
        }
    );
}
