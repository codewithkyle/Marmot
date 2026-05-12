#[cfg(test)]
mod test;

use crate::{
    lexer::{Token, TokenKind},
    renderer::{ImageFit, LineBreakMode, TextAlign, TextFit, VerticalAlign},
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub version: String,
    pub page: Page,
    pub slots: Vec<SlotDecl>,
    pub draw: Vec<DrawOp>,
    pub fonts: Vec<FontDecl>,
    pub assets: Vec<AssetDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontDecl {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetDecl {
    pub name: String,
    pub path: String,
    pub ty: AssetType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssetType {
    Image,
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
    Concat(Vec<TextValue>),
    UpperCase(Box<TextValue>),
    LowerCase(Box<TextValue>),
    TitleCase(Box<TextValue>),
    Capitalize(Box<TextValue>),
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
    SetFontSize {
        size: NumberValue,
    },
    SetTextAlignment {
        align: TextAlign,
    },
    SetVerticalAlignment {
        align: VerticalAlign,
    },
    SetLineBreakMode {
        line_break: LineBreakMode,
    },
    SetTextFit {
        fit: TextFit,
    },
    SetTextFitMinSize {
        min: NumberValue,
    },
    SetTextFitMaxSize {
        max: NumberValue,
    },
    SetFontFamily {
        font: TextValue,
    },
    SetImageFit {
        fit: ImageFit,
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
    Image {
        asset: TextValue,
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
    InvalidConcatOperation {
        line: usize,
        column: usize,
    },
    MustBeLiteralNumber {
        slot: String,
        line: usize,
        column: usize,
    },
    UnpaintedPath {
        line: usize,
        column: usize,
    },
    UnusedStackValues {
        count: usize,
        line: usize,
        column: usize,
    },
    InvalidNumberOperand {
        operator: String,
        operand: String,
        value: f64,
        expected: String,
        line: usize,
        column: usize,
    },
    UnknownAssetType {
        found: String,
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
    ExpectedString {
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
        let fonts = self.parse_optional_fonts()?;
        let assets = self.parse_optional_assets()?;

        let slot_lookup = Self::build_slot_lookup(&slots);

        let draw = self.parse_draw(&slot_lookup)?;

        self.expect_eof()?;

        Ok(Template {
            version,
            page,
            slots,
            draw,
            fonts,
            assets,
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

    fn try_consume_word(&mut self, expected: &str) -> bool {
        match &self.peek().kind {
            TokenKind::Word(value) if value == expected => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::String(value) => Ok(value.clone()),
            found => Err(ParseError::ExpectedString {
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn expect_asset_word(&mut self) -> Result<AssetType, ParseError> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Word(value) => match value.as_str() {
                "image" => Ok(AssetType::Image),
                found => Err(ParseError::UnknownAssetType {
                    found: found.to_string(),
                    line: token.line,
                    column: token.column,
                }),
            },
            found => Err(ParseError::ExpectedAnyWord {
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
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

    fn validate_literal_range(
        value: &NumberValue,
        operator: &str,
        operand: &str,
        min: f64,
        max: f64,
        token: &Token,
    ) -> Result<(), ParseError> {
        match value {
            NumberValue::Literal(n) if !n.is_finite() || *n < min || *n > max => {
                Err(ParseError::InvalidNumberOperand {
                    operator: operator.to_string(),
                    operand: operand.to_string(),
                    value: *n,
                    expected: format!("{min}..={max}"),
                    line: token.line,
                    column: token.column,
                })
            }
            _ => Ok(()),
        }
    }

    fn validate_literal_positive(
        value: &NumberValue,
        operator: &str,
        operand: &str,
        token: &Token,
    ) -> Result<(), ParseError> {
        match value {
            NumberValue::Literal(n) if !n.is_finite() || *n <= 0.0 => {
                Err(ParseError::InvalidNumberOperand {
                    operator: operator.to_string(),
                    operand: operand.to_string(),
                    value: *n,
                    expected: format!("> 0"),
                    line: token.line,
                    column: token.column,
                })
            }
            _ => Ok(()),
        }
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
                TokenKind::Word(word) if word == "concat" => {
                    Self::require_stack(&stack, "concat", 1, token)?;
                    let count_value = Self::pop_number(&mut stack, "concat", token)?;
                    let count = match count_value {
                        NumberValue::Literal(n)
                            if n.is_finite() && n >= 0.0 && n.fract() == 0.0 =>
                        {
                            n as usize
                        }
                        NumberValue::Literal(n) => {
                            return Err(ParseError::InvalidNumberOperand {
                                operator: "concat".to_string(),
                                operand: ">= 0.0".to_string(),
                                value: n,
                                expected: "non-negative integer".to_string(),
                                line: token.line,
                                column: token.column,
                            });
                        }
                        NumberValue::Slot(name) => {
                            return Err(ParseError::MustBeLiteralNumber {
                                slot: name.to_string(),
                                line: token.line,
                                column: token.column,
                            });
                        }
                    };
                    Self::require_stack(&stack, "concat", count, token)?;
                    let mut values: Vec<TextValue> = Vec::new();
                    for _ in 0..count {
                        let value = Self::pop_string(&mut stack, "concat", token)?;
                        match value {
                            TextValue::Concat(_) => {
                                return Err(ParseError::InvalidConcatOperation {
                                    line: token.line,
                                    column: token.column,
                                });
                            }
                            _ => {}
                        };
                        values.push(value);
                    }
                    values.reverse();
                    stack.push(StackValue::Text(TextValue::Concat(values)));
                }
                TokenKind::Word(word) if word == "uppercase" => {
                    Self::require_stack(&stack, "uppercase", 1, token)?;
                    let value = Self::pop_string(&mut stack, "uppercase", token)?;
                    stack.push(StackValue::Text(TextValue::UpperCase(Box::new(value))));
                }
                TokenKind::Word(word) if word == "lowercase" => {
                    Self::require_stack(&stack, "lowercase", 1, token)?;
                    let value = Self::pop_string(&mut stack, "lowercase", token)?;
                    stack.push(StackValue::Text(TextValue::LowerCase(Box::new(value))));
                }
                TokenKind::Word(word) if word == "capitalize" => {
                    Self::require_stack(&stack, "capitalize", 1, token)?;
                    let value = Self::pop_string(&mut stack, "capitalize", token)?;
                    stack.push(StackValue::Text(TextValue::Capitalize(Box::new(value))));
                }
                TokenKind::Word(word) if word == "titlecase" => {
                    Self::require_stack(&stack, "title", 1, token)?;
                    let value = Self::pop_string(&mut stack, "title", token)?;
                    stack.push(StackValue::Text(TextValue::TitleCase(Box::new(value))));
                }
                TokenKind::Word(word) if ImageFit::from_word(word).is_some() => {
                    let fit = ImageFit::from_word(word).unwrap();
                    self.expect_word("imagefit")?;
                    ops.push(DrawOp::SetImageFit { fit });
                }
                TokenKind::Word(word) if word == "image" => {
                    Self::require_stack(&stack, "image", 5, token)?;
                    let height = Self::pop_number(&mut stack, "image", token)?;
                    let width = Self::pop_number(&mut stack, "image", token)?;
                    let y = Self::pop_number(&mut stack, "image", token)?;
                    let x = Self::pop_number(&mut stack, "image", token)?;
                    let asset = Self::pop_string(&mut stack, "image", token)?;

                    Self::validate_literal_positive(&width, "image", "width", token)?;
                    Self::validate_literal_positive(&height, "image", "height", token)?;

                    ops.push(DrawOp::Image {
                        asset,
                        x,
                        y,
                        width,
                        height,
                    });
                }
                TokenKind::Word(word) if word == "font" => {
                    Self::require_stack(&stack, "font", 1, token)?;
                    let font = Self::pop_string(&mut stack, "font", token)?;
                    ops.push(DrawOp::SetFontFamily { font });
                }
                TokenKind::Word(word) if TextAlign::from_word(word).is_some() => {
                    let align = TextAlign::from_word(word).unwrap();
                    self.expect_word("align")?;
                    ops.push(DrawOp::SetTextAlignment { align });
                }
                TokenKind::Word(word) if VerticalAlign::from_word(word).is_some() => {
                    let align = VerticalAlign::from_word(word).unwrap();
                    self.expect_word("valign")?;
                    ops.push(DrawOp::SetVerticalAlignment { align });
                }
                TokenKind::Word(word) if LineBreakMode::from_word(word).is_some() => {
                    let break_mode = LineBreakMode::from_word(word).unwrap();
                    self.expect_word("wrap")?;
                    ops.push(DrawOp::SetLineBreakMode {
                        line_break: break_mode,
                    });
                }
                TokenKind::Word(word) if TextFit::from_word(word).is_some() => {
                    let fit = TextFit::from_word(word).unwrap();
                    self.expect_word("textfit")?;
                    ops.push(DrawOp::SetTextFit { fit });
                }
                TokenKind::Word(word) if word == "textfitmin" => {
                    Self::require_stack(&stack, "textfitmin", 1, token)?;
                    let size = Self::pop_number(&mut stack, "textfitmin", token)?;
                    Self::validate_literal_positive(&size, "textfitmin", "size", token)?;
                    ops.push(DrawOp::SetTextFitMinSize { min: size });
                }
                TokenKind::Word(word) if word == "textfitmax" => {
                    Self::require_stack(&stack, "textfitmax", 1, token)?;
                    let size = Self::pop_number(&mut stack, "textfitmax", token)?;
                    Self::validate_literal_positive(&size, "textfitmax", "size", token)?;
                    ops.push(DrawOp::SetTextFitMaxSize { max: size });
                }
                TokenKind::Word(word) if word == "fontsize" => {
                    Self::require_stack(&stack, "fontsize", 1, token)?;
                    let size = Self::pop_number(&mut stack, "fontsize", token)?;
                    Self::validate_literal_positive(&size, "fontsize", "size", token)?;
                    ops.push(DrawOp::SetFontSize { size });
                }
                TokenKind::Word(word) if word == "textbox" => {
                    Self::require_stack(&stack, "textbox", 5, token)?;
                    let height = Self::pop_number(&mut stack, "textbox", token)?;
                    let width = Self::pop_number(&mut stack, "textbox", token)?;
                    let y = Self::pop_number(&mut stack, "textbox", token)?;
                    let x = Self::pop_number(&mut stack, "textbox", token)?;
                    let text = Self::pop_string(&mut stack, "textbox", token)?;

                    Self::validate_literal_positive(&width, "textbox", "width", token)?;
                    Self::validate_literal_positive(&height, "textbox", "height", token)?;

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

                    Self::validate_literal_range(&r, "rgb", "r", 0.0, 1.0, token)?;
                    Self::validate_literal_range(&g, "rgb", "g", 0.0, 1.0, token)?;
                    Self::validate_literal_range(&b, "rgb", "b", 0.0, 1.0, token)?;

                    ops.push(DrawOp::SetRgb { r, g, b });
                }
                TokenKind::Word(word) if word == "strokewidth" => {
                    Self::require_stack(&stack, "strokewidth", 1, token)?;
                    let width = Self::pop_number(&mut stack, "strokewidth", token)?;

                    Self::validate_literal_positive(&width, "strokewidth", "width", token)?;

                    ops.push(DrawOp::SetStrokeWidth { width });
                }
                TokenKind::Word(word) if word == "line" => {
                    if current_path_kind.is_some() {
                        return Err(ParseError::UnpaintedPath {
                            line: token.line,
                            column: token.column,
                        });
                    }

                    Self::require_stack(&stack, "line", 4, token)?;
                    let y2 = Self::pop_number(&mut stack, "line", token)?;
                    let x2 = Self::pop_number(&mut stack, "line", token)?;
                    let y1 = Self::pop_number(&mut stack, "line", token)?;
                    let x1 = Self::pop_number(&mut stack, "line", token)?;

                    ops.push(DrawOp::LinePath { x1, y1, x2, y2 });
                    current_path_kind = Some(CurrentPathKind::Line);
                }
                TokenKind::Word(word) if word == "rect" => {
                    if current_path_kind.is_some() {
                        return Err(ParseError::UnpaintedPath {
                            line: token.line,
                            column: token.column,
                        });
                    }

                    Self::require_stack(&stack, "rect", 4, token)?;
                    let height = Self::pop_number(&mut stack, "rect", token)?;
                    let width = Self::pop_number(&mut stack, "rect", token)?;
                    let y = Self::pop_number(&mut stack, "rect", token)?;
                    let x = Self::pop_number(&mut stack, "rect", token)?;

                    Self::validate_literal_positive(&width, "rect", "width", token)?;
                    Self::validate_literal_positive(&height, "rect", "height", token)?;

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

        if !stack.is_empty() {
            let token = self.peek();
            return Err(ParseError::UnusedStackValues {
                count: stack.len(),
                line: token.line,
                column: token.column,
            });
        }
        if current_path_kind.is_some() {
            let token = self.peek();
            return Err(ParseError::UnpaintedPath {
                line: token.line,
                column: token.column,
            });
        }

        self.expect_word("end")?;
        Ok(ops)
    }

    fn parse_optional_fonts(&mut self) -> Result<Vec<FontDecl>, ParseError> {
        if !self.check_word("fonts") {
            return Ok(Vec::new());
        }

        self.parse_fonts()
    }

    fn parse_optional_assets(&mut self) -> Result<Vec<AssetDecl>, ParseError> {
        if !self.check_word("assets") {
            return Ok(Vec::new());
        }

        self.parse_assets()
    }

    fn parse_assets(&mut self) -> Result<Vec<AssetDecl>, ParseError> {
        self.expect_word("assets")?;
        self.expect_word("begin")?;

        let mut assets = Vec::new();

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "assets block".to_string(),
                });
            }

            let asset = self.parse_asset_decl()?;
            assets.push(asset);
        }

        self.expect_word("end")?;
        Ok(assets)
    }

    fn parse_asset_decl(&mut self) -> Result<AssetDecl, ParseError> {
        let name = self.expect_any_word()?;
        let ty = self.expect_asset_word()?;
        let path = self.expect_string()?;

        Ok(AssetDecl { name, path, ty })
    }

    fn parse_font_decl(&mut self) -> Result<FontDecl, ParseError> {
        let name = self.expect_any_word()?;
        let path = self.expect_string()?;

        Ok(FontDecl { name, path })
    }

    fn parse_fonts(&mut self) -> Result<Vec<FontDecl>, ParseError> {
        self.expect_word("fonts")?;
        self.expect_word("begin")?;

        let mut fonts = Vec::new();

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "fonts block".to_string(),
                });
            }

            let font = self.parse_font_decl()?;
            fonts.push(font);
        }

        self.expect_word("end")?;
        Ok(fonts)
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
        let is_required = self.try_consume_word("required");

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
