use super::*;
use crate::lexer::Lexer;

#[test]
fn parses_header_and_page() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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
            line: 12,
            column: 1,
        }
    );
}

#[test]
fn parses_template_without_slots() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      1 0 0 rgb
      2 strokewidth
      0 0 10 10 line stroke
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      1 0 rgb
  end
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
            line: 10,
            column: 11,
        }
    );
}

#[test]
fn parses_cmyk_command() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0.1 0.2 0.3 0.4 cmyk
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::SetCmyk {
            c: NumberValue::Literal(0.1),
            m: NumberValue::Literal(0.2),
            y: NumberValue::Literal(0.3),
            k: NumberValue::Literal(0.4),
        }]
    );
}

#[test]
fn errors_when_cmyk_has_too_few_values() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0.1 0.2 0.3 cmyk
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::StackUnderflow {
            operator: "cmyk".to_string(),
            expected: 4,
            actual: 3,
            line: 10,
            column: 19,
        }
    );
}

#[test]
fn errors_when_line_has_too_few_values() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0 0 10 line
  end
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
            line: 10,
            column: 14,
        }
    );
}

#[test]
fn parses_static_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (Hello world) 0 0 100 100 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0 0 100 100 textbox
  end
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
            line: 10,
            column: 19,
        }
    );
}

#[test]
fn errors_when_textbox_text_is_not_string() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      1 0 0 100 100 textbox
  end
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
            line: 10,
            column: 21,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(x) 0 10 10 rect stroke
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0 0 10 10 rect fill
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(missing) 0 0 100 100 textbox
  end
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
            line: 10,
            column: 7,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(product_name) 0 0 100 100 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
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
fn allows_slot_rgb_component() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  r decimal
end

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(r) 0 0 rgb
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::SetRgb {
            r: NumberValue::Slot("r".to_string()),
            g: NumberValue::Literal(0.0),
            b: NumberValue::Literal(0.0),
        }]
    );
}

#[test]
fn allows_slot_cmyk_component() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  c decimal
end

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(c) 0 0 0 cmyk
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::SetCmyk {
            c: NumberValue::Slot("c".to_string()),
            m: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            k: NumberValue::Literal(0.0),
        }]
    );
}

#[test]
fn errors_on_zero_rect_width() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0 0 0 10 rect fill
  end
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
            line: 10,
            column: 16,
        }
    );
}

#[test]
fn allows_zero_line_coordinates() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      0 0 10 10 line stroke
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
  end
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

#[test]
fn errors_on_invalid_header_comment() {
    let source = r#"% hello
page 612 792
draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::InvalidHeader {
            value: "hello".to_string(),
            line: 1,
            column: 1,
        }
    );
}

#[test]
fn errors_when_page_keyword_is_wrong() {
    let source = r#"%!PSL 0.1
size 612 792
draw begin
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::ExpectedWord {
            expected: "page".to_string(),
            found: TokenKind::Word("size".to_string()),
            line: 2,
            column: 1,
        }
    );
}

#[test]
fn errors_when_draw_begin_keyword_is_missing() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw start
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::ExpectedWord {
            expected: "begin".to_string(),
            found: TokenKind::Word("start".to_string()),
            line: 8,
            column: 6,
        }
    );
}

#[test]
fn errors_on_unexpected_eof_in_draw_block() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnexpectedEof {
            context: "draw block".to_string(),
        }
    );
}

#[test]
fn errors_on_unexpected_eof_in_slots_block() {
    let source = r#"%!PSL 0.1
page 612 792
slots begin
  product_name string
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnexpectedEof {
            context: "slots block".to_string(),
        }
    );
}

#[test]
fn errors_on_unexpected_eof_in_fonts_block() {
    let source = r#"%!PSL 0.1
page 612 792
fonts begin
  helvetica "fonts/Helvetica.ttf"
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnexpectedEof {
            context: "fonts block".to_string(),
        }
    );
}

#[test]
fn errors_when_slot_name_is_not_a_word() {
    let source = r#"%!PSL 0.1
page 612 792

slots begin
  123 string
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
        ParseError::ExpectedAnyWord {
            found: TokenKind::Number(123.0),
            line: 5,
            column: 3,
        }
    );
}

#[test]
fn errors_when_font_name_is_not_a_word() {
    let source = r#"%!PSL 0.1
page 612 792

fonts begin
  "helvetica" "fonts/Helvetica.ttf"
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
        ParseError::ExpectedAnyWord {
            found: TokenKind::String("helvetica".to_string()),
            line: 5,
            column: 3,
        }
    );
}

#[test]
fn errors_on_unexpected_word_in_draw_block() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    banana
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnexpectedDrawToken {
            found: TokenKind::Word("banana".to_string()),
            line: 10,
            column: 5,
        }
    );
}

#[test]
fn errors_on_fill_without_current_path() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    fill
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::NoCurrentPath {
            operator: "fill".to_string(),
            line: 10,
            column: 5,
        }
    );
}

#[test]
fn errors_on_stroke_without_current_path() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    stroke
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::NoCurrentPath {
            operator: "stroke".to_string(),
            line: 10,
            column: 5,
        }
    );
}

#[test]
fn errors_on_filling_line_path() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    0 0 10 10 line fill
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::CannotFillPath {
            path: "line".to_string(),
            line: 10,
            column: 20,
        }
    );
}

#[test]
fn errors_on_unpainted_path_before_new_path() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    0 0 10 10 rect 1 1 2 2 rect fill
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnpaintedPath {
            line: 10,
            column: 28
        }
    );
}

#[test]
fn errors_on_unpainted_path_at_end_of_draw_block() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    0 0 10 10 rect
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(err, ParseError::UnpaintedPath { line: 11, column: 3 });
}

#[test]
fn errors_on_unused_stack_values_in_draw_block() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
    1
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert_eq!(
        err,
        ParseError::UnusedStackValues {
            count: 1,
            line: 11,
            column: 3,
        }
    );
}

#[test]
fn parses_draw_text_style_operators() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (Helvetica-Bold) font
      12 fontsize
      center align
      middle valign
      char wrap
      fit textfit
      8 textfitmin
      40 textfitmax
      (Hello) 10 20 200 40 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![
            DrawOp::SetFontFamily {
                font: TextValue::Literal("Helvetica-Bold".to_string()),
            },
            DrawOp::SetFontSize {
                size: NumberValue::Literal(12.0),
            },
            DrawOp::SetTextAlignment {
                align: TextAlign::Center,
            },
            DrawOp::SetVerticalAlignment {
                align: VerticalAlign::Middle,
            },
            DrawOp::SetLineBreakMode {
                line_break: LineBreakMode::Char,
            },
            DrawOp::SetTextFit { fit: TextFit::Fit },
            DrawOp::SetTextFitMinSize {
                min: NumberValue::Literal(8.0),
            },
            DrawOp::SetTextFitMaxSize {
                max: NumberValue::Literal(40.0),
            },
            DrawOp::TextBox {
                text: TextValue::Literal("Hello".to_string()),
                x: NumberValue::Literal(10.0),
                y: NumberValue::Literal(20.0),
                width: NumberValue::Literal(200.0),
                height: NumberValue::Literal(40.0),
            },
        ]
    );
}

#[test]
fn parses_image_draw_op() {
    let source = r#"%!PSL 0.1
page 200 100
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      "logo" 10 20 30 40 image
  end
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();
    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Image {
            asset: TextValue::Literal("logo".to_string()),
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        }]
    );
}
#[test]
fn errors_when_image_asset_operand_is_not_string() {
    let source = r#"%!PSL 0.1
page 200 100
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      1 10 20 30 40 image
  end
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnexpectedStackValue {
            operator,
            expected,
            found,
            ..
        } if operator == "image" && expected == "string" && found == "number"
    ));
}

#[test]
fn parses_imagefit_commands() {
    let source = r#"%!PSL 0.1
page 100 100
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      contain imagefit
      cover imagefit
      stretch imagefit
  end
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();
    assert_eq!(
        template.draw_frames[0].ops,
        vec![
            DrawOp::SetImageFit {
                fit: ImageFit::Contain
            },
            DrawOp::SetImageFit {
                fit: ImageFit::Cover
            },
            DrawOp::SetImageFit {
                fit: ImageFit::Stretch
            },
        ]
    );
}

#[test]
fn parses_concat_for_textbox() {
    let source = r#"%!PSL 0.1
page 612 792
slots begin
  B string required
  G string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (BUY ) $(B) ( GET ) $(G) 4 concat 0 0 100 25 textbox
  end
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();
    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::TextBox {
            text: TextValue::Concat(vec![
                TextValue::Literal("BUY ".to_string()),
                TextValue::Slot("B".to_string()),
                TextValue::Literal(" GET ".to_string()),
                TextValue::Slot("G".to_string()),
            ]),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(25.0),
        }]
    );
}
#[test]
fn errors_when_concat_count_is_slot() {
    let source = r#"%!PSL 0.1
page 612 792
slots begin
  n int required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (A) $(n) concat 0 0 100 25 textbox
  end
end
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();
    assert!(matches!(
        err,
        ParseError::MustBeLiteralNumber { slot, .. } if slot == "n"
    ));
}

#[test]
fn parses_uppercase_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (hELLo) uppercase 0 0 100 25 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::TextBox {
            text: TextValue::UpperCase(Box::new(TextValue::Literal("hELLo".to_string()))),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(25.0),
        }]
    );
}

#[test]
fn parses_lowercase_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (hELLo) lowercase 0 0 100 25 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::TextBox {
            text: TextValue::LowerCase(Box::new(TextValue::Literal("hELLo".to_string()))),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(25.0),
        }]
    );
}

#[test]
fn parses_titlecase_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (hELLo wORLd) titlecase 0 0 100 25 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::TextBox {
            text: TextValue::TitleCase(Box::new(TextValue::Literal(
                "hELLo wORLd".to_string(),
            ))),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(25.0),
        }]
    );
}

#[test]
fn parses_capitalize_textbox() {
    let source = r#"%!PSL 0.1
page 612 792

frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      (hELLo wORLd) capitalize 0 0 100 25 textbox
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::TextBox {
            text: TextValue::Capitalize(Box::new(TextValue::Literal(
                "hELLo wORLd".to_string(),
            ))),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(25.0),
        }]
    );
}

#[test]
fn parses_code39_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  sku string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(sku) c39 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("sku".to_string()),
            symbology: BarcodeSymbology::Code39,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn errors_when_barcode_data_operand_is_not_string() {
    let source = r#"%!PSL 0.1
page 300 200
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      123 c39 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let err = parser.parse_template().unwrap_err();

    assert!(matches!(
        err,
        ParseError::UnexpectedStackValue {
            operator,
            expected,
            found,
            ..
        } if operator == "barcode" && expected == "string" && found == "number"
    ));
}

#[test]
fn parses_code128a_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  sku string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(sku) c128a 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("sku".to_string()),
            symbology: BarcodeSymbology::Code128A,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn parses_code128b_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  sku string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(sku) c128b 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("sku".to_string()),
            symbology: BarcodeSymbology::Code128B,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn parses_code128c_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  sku string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(sku) c128c 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("sku".to_string()),
            symbology: BarcodeSymbology::Code128C,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn parses_upca_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  upc string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(upc) upca 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("upc".to_string()),
            symbology: BarcodeSymbology::UPCA,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn parses_ean13_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  ean string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(ean) ean13 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("ean".to_string()),
            symbology: BarcodeSymbology::EAN13,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn parses_ean8_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  ean string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(ean) ean8 20 20 220 50 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("ean".to_string()),
            symbology: BarcodeSymbology::EAN8,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(220.0),
            height: NumberValue::Literal(50.0),
        }]
    );
}

#[test]
fn parses_qr_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  url string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(url) qr 20 20 120 120 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("url".to_string()),
            symbology: BarcodeSymbology::QR,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(120.0),
            height: NumberValue::Literal(120.0),
        }]
    );
}

#[test]
fn parses_datamatrix_barcode_draw_op() {
    let source = r#"%!PSL 0.1
page 300 200
slots begin
  payload string required
end
frames begin
  1 FRAME_1
end

draw begin
  frame 1 begin
      $(payload) datamatrix 20 20 120 120 barcode
  end
end
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let template = parser.parse_template().unwrap();

    assert_eq!(
        template.draw_frames[0].ops,
        vec![DrawOp::Barcode {
            value: TextValue::Slot("payload".to_string()),
            symbology: BarcodeSymbology::DataMatrix,
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(120.0),
            height: NumberValue::Literal(120.0),
        }]
    );
}
