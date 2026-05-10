use crate::lexer::{Token, TokenKind};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub version: String,
    pub page: Page,
    pub slots: Vec<SlotDecl>,
    pub draw: Vec<DrawOp>,
}

#[derive(Debug, Clone, PartialEq)]
enum StackValue {
    Number(NumberValue),
    Text(TextValue),
}

impl StackValue {
    fn type_name(&self) -> &'static str {
        match self {
            StackValue::Number(_) => "number",
            StackValue::Text(_) => "string",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CurrentPathKind {
    Line,
    Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextValue {
    Literal(String),
    Slot(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberValue {
    Literal(f64),
    Slot(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawOp {
    SetRgb {
        r: NumberValue,
        g: NumberValue,
        b: NumberValue,
    },
    SetStrokeWidth {
        width: NumberValue,
    },
    LinePath {
        x1: NumberValue,
        y1: NumberValue,
        x2: NumberValue,
        y2: NumberValue,
    },
    RectPath {
        x: NumberValue,
        y: NumberValue,
        width: NumberValue,
        height: NumberValue,
    },
    Stroke,
    Fill,
    TextBox {
        text: TextValue,
        x: NumberValue,
        y: NumberValue,
        width: NumberValue,
        height: NumberValue,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotDecl {
    pub name: String,
    pub ty: SlotType,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SlotType {
    String,
    Int,
    Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    InvalidSlotUsage {
        name: String,
        expected: String,
        actual: String,
        line: usize,
        column: usize,
    },
    UnknownSlot {
        name: String,
        line: usize,
        column: usize,
    },
    UnexpectedStackValue {
        operator: String,
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },
    CannotFillPath {
        path: String,
        line: usize,
        column: usize,
    },
    NoCurrentPath {
        operator: String,
        line: usize,
        column: usize,
    },
    StackUnderflow {
        operator: String,
        expected: usize,
        actual: usize,
        line: usize,
        column: usize,
    },
    UnexpectedDrawToken {
        found: TokenKind,
        line: usize,
        column: usize,
    },
    InvalidSlotType {
        value: String,
    },
    UnexpectedEof {
        context: String,
    },
    ExpectedHeader {
        line: usize,
        column: usize,
    },
    InvalidHeader {
        value: String,
        line: usize,
        column: usize,
    },
    ExpectedWord {
        expected: String,
        found: TokenKind,
        line: usize,
        column: usize,
    },
    ExpectedAnyWord {
        found: TokenKind,
        line: usize,
        column: usize,
    },
    ExpectedNumber {
        found: TokenKind,
        line: usize,
        column: usize,
    },
    ExpectedEof {
        found: TokenKind,
        line: usize,
        column: usize,
    },
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_template(&mut self) -> Result<Template, ParseError> {
        let version = self.parse_header()?;
        let page = self.parse_page()?;

        let slots = self.parse_optional_slots()?;
        let slot_lookup = Self::build_slot_lookup(&slots);

        let draw = self.parse_draw(&slot_lookup)?;

        self.expect_eof()?;

        Ok(Template {
            version,
            page,
            slots,
            draw,
        })
    }

    fn check_word(&self, expected: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Word(value) if value == expected)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn is_eof(&self) -> bool {
        matches!(self.tokens[self.current].kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if self.is_eof() {
            return &self.tokens[self.current];
        }
        self.current += 1;
        &self.tokens[self.current - 1]
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Word(value) if value == expected => Ok(()),
            found => Err(ParseError::ExpectedWord {
                expected: expected.to_string(),
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn safe_expect_word(&mut self, expected: &str) -> bool {
        match &self.peek().kind {
            TokenKind::Word(value) if value == expected => {
                self.advance();
                true
            }
            _ => {
                false
            }
        }
    }

    fn expect_any_word(&mut self) -> Result<String, ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Word(value) => Ok(value.clone()),
            found => Err(ParseError::ExpectedAnyWord {
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn expect_number(&mut self) -> Result<f64, ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Number(value) => Ok(*value),
            found => Err(ParseError::ExpectedNumber {
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn expect_eof(&mut self) -> Result<(), ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Eof => Ok(()),
            found => Err(ParseError::ExpectedEof {
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn pop_number(
        stack: &mut Vec<StackValue>,
        operator: &str,
        token: &Token,
    ) -> Result<NumberValue, ParseError> {
        let Some(value) = stack.pop() else {
            return Err(ParseError::StackUnderflow {
                operator: operator.to_string(),
                expected: 1,
                actual: 0,
                line: token.line,
                column: token.column,
            });
        };

        match value {
            StackValue::Number(number) => Ok(number),
            other => Err(ParseError::UnexpectedStackValue {
                operator: operator.to_string(),
                expected: "number".to_string(),
                found: other.type_name().to_string(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn pop_string(
        stack: &mut Vec<StackValue>,
        operator: &str,
        token: &Token,
    ) -> Result<TextValue, ParseError> {
        let Some(value) = stack.pop() else {
            return Err(ParseError::StackUnderflow {
                operator: operator.to_string(),
                expected: 1,
                actual: 0,
                line: token.line,
                column: token.column,
            });
        };

        match value {
            StackValue::Text(value) => Ok(value),
            other => Err(ParseError::UnexpectedStackValue {
                operator: operator.to_string(),
                expected: "string".to_string(),
                found: other.type_name().to_string(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn require_stack(
        stack: &[StackValue],
        operator: &str,
        expected: usize,
        token: &Token,
    ) -> Result<(), ParseError> {
        if stack.len() < expected {
            return Err(ParseError::StackUnderflow {
                operator: operator.to_string(),
                expected,
                actual: stack.len(),
                line: token.line,
                column: token.column,
            });
        }
        Ok(())
    }

    fn build_slot_lookup(slots: &[SlotDecl]) -> HashMap<String, SlotType> {
        slots
            .iter()
            .map(|slot| (slot.name.clone(), slot.ty.clone()))
            .collect()
    }

    fn parse_draw(&mut self, slots: &HashMap<String, SlotType>) -> Result<Vec<DrawOp>, ParseError> {
        self.expect_word("draw")?;
        self.expect_word("begin")?;

        let mut stack: Vec<StackValue> = Vec::new();
        let mut ops: Vec<DrawOp> = Vec::new();
        let mut current_path_kind: Option<CurrentPathKind> = None;

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "draw block".to_string(),
                });
            }

            let token = self.advance();

            match &token.kind {
                TokenKind::Number(value) => {
                    stack.push(StackValue::Number(NumberValue::Literal(*value)));
                }
                TokenKind::String(value) => {
                    stack.push(StackValue::Text(TextValue::Literal(value.clone())));
                }
                TokenKind::Slot(slot_name) => {
                    let Some(slot_ty) = slots.get(slot_name) else {
                        return Err(ParseError::UnknownSlot {
                            name: slot_name.clone(),
                            line: token.line,
                            column: token.column,
                        });
                    };

                    match slot_ty {
                        SlotType::String => {
                            stack.push(StackValue::Text(TextValue::Slot(slot_name.clone())));
                        }
                        SlotType::Int | SlotType::Decimal => {
                            stack.push(StackValue::Number(NumberValue::Slot(slot_name.clone())));
                        }
                    }
                }
                TokenKind::Word(word) if word == "textbox" => {
                    Self::require_stack(&stack, "textbox", 5, token)?;
                    let height = Self::pop_number(&mut stack, "textbox", token)?;
                    let width = Self::pop_number(&mut stack, "textbox", token)?;
                    let y = Self::pop_number(&mut stack, "textbox", token)?;
                    let x = Self::pop_number(&mut stack, "textbox", token)?;
                    let text = Self::pop_string(&mut stack, "textbox", token)?;

                    ops.push(DrawOp::TextBox {
                        text,
                        x,
                        y,
                        width,
                        height,
                    });
                }
                TokenKind::Word(word) if word == "rgb" => {
                    Self::require_stack(&stack, "rgb", 3, token)?;
                    let b = Self::pop_number(&mut stack, "rgb", token)?;
                    let g = Self::pop_number(&mut stack, "rgb", token)?;
                    let r = Self::pop_number(&mut stack, "rgb", token)?;

                    ops.push(DrawOp::SetRgb { r, g, b });
                }
                TokenKind::Word(word) if word == "strokewidth" => {
                    Self::require_stack(&stack, "strokewidth", 1, token)?;
                    let width = Self::pop_number(&mut stack, "strokewidth", token)?;

                    ops.push(DrawOp::SetStrokeWidth { width });
                }
                TokenKind::Word(word) if word == "line" => {
                    Self::require_stack(&stack, "line", 4, token)?;
                    let y2 = Self::pop_number(&mut stack, "line", token)?;
                    let x2 = Self::pop_number(&mut stack, "line", token)?;
                    let y1 = Self::pop_number(&mut stack, "line", token)?;
                    let x1 = Self::pop_number(&mut stack, "line", token)?;

                    ops.push(DrawOp::LinePath { x1, y1, x2, y2 });
                    current_path_kind = Some(CurrentPathKind::Line);
                }
                TokenKind::Word(word) if word == "rect" => {
                    Self::require_stack(&stack, "rect", 4, token)?;
                    let height = Self::pop_number(&mut stack, "rect", token)?;
                    let width = Self::pop_number(&mut stack, "rect", token)?;
                    let y = Self::pop_number(&mut stack, "rect", token)?;
                    let x = Self::pop_number(&mut stack, "rect", token)?;

                    ops.push(DrawOp::RectPath {
                        x,
                        y,
                        width,
                        height,
                    });
                    current_path_kind = Some(CurrentPathKind::Rect);
                }
                TokenKind::Word(word) if word == "stroke" => {
                    if current_path_kind.is_none() {
                        return Err(ParseError::NoCurrentPath {
                            operator: "stroke".to_string(),
                            line: token.line,
                            column: token.column,
                        });
                    }
                    ops.push(DrawOp::Stroke);
                    current_path_kind = None;
                }
                TokenKind::Word(word) if word == "fill" => match current_path_kind {
                    Some(CurrentPathKind::Rect) => {
                        ops.push(DrawOp::Fill);
                        current_path_kind = None;
                    }
                    Some(CurrentPathKind::Line) => {
                        return Err(ParseError::CannotFillPath {
                            path: "line".to_string(),
                            line: token.line,
                            column: token.column,
                        });
                    }
                    None => {
                        return Err(ParseError::NoCurrentPath {
                            operator: "fill".to_string(),
                            line: token.line,
                            column: token.column,
                        });
                    }
                },
                found => {
                    return Err(ParseError::UnexpectedDrawToken {
                        found: found.clone(),
                        line: token.line,
                        column: token.column,
                    });
                }
            }
        }

        self.expect_word("end")?;
        Ok(ops)
    }

    fn parse_optional_slots(&mut self) -> Result<Vec<SlotDecl>, ParseError> {
        if !self.check_word("slots") {
            return Ok(Vec::new());
        }
        self.parse_slots()
    }

    fn parse_slots(&mut self) -> Result<Vec<SlotDecl>, ParseError> {
        self.expect_word("slots")?;
        self.expect_word("begin")?;

        let mut slots = Vec::new();

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "slots block".to_string(),
                });
            }
            let slot = self.parse_slot_decl()?;
            slots.push(slot);
        }

        self.expect_word("end")?;

        Ok(slots)
    }

    fn parse_slot_decl(&mut self) -> Result<SlotDecl, ParseError> {
        let name = self.expect_any_word()?;
        let ty_word = self.expect_any_word()?;
        let is_required = self.safe_expect_word("required");

        let ty = match ty_word.as_str() {
            "string" => SlotType::String,
            "int" => SlotType::Int,
            "decimal" => SlotType::Decimal,
            other => {
                return Err(ParseError::InvalidSlotType {
                    value: other.to_string(),
                });
            }
        };

        Ok(SlotDecl {
            name,
            ty,
            required: is_required,
        })
    }

    fn parse_header(&mut self) -> Result<String, ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Comment(value) => {
                let Some(version) = value.strip_prefix("!PSL ") else {
                    return Err(ParseError::InvalidHeader {
                        value: value.clone(),
                        line: token.line,
                        column: token.column,
                    });
                };
                Ok(version.trim().to_string())
            }
            _ => Err(ParseError::ExpectedHeader {
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn parse_page(&mut self) -> Result<Page, ParseError> {
        self.expect_word("page")?;

        let width = self.expect_number()?;
        let height = self.expect_number()?;

        Ok(Page { width, height })
    }
}

#[cfg(test)]
mod tests {
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
  0 0 10 10 line
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
}
