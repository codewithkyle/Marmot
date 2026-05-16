#[cfg(test)]
mod test;

use crate::{
    lexer::{Token, TokenKind},
    renderer::{ImageFit, LineBreakMode, TextAlign, TextFit, VerticalAlign},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct FrameDecl {
    pub index: u32,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameDrawBlock {
    pub index: u32,
    pub ops: Vec<DrawOp>,
}

impl Default for FrameDrawBlock {
    fn default() -> Self {
        Self {
            index: 0,
            ops: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BarcodeSymbology {
    Code39,
    Code128A,
    Code128B,
    Code128C,
    UPCA,
    EAN13,
    EAN8,
    MSI,
    QR,
    DataMatrix,
}

impl BarcodeSymbology {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "c39" => Some(Self::Code39),
            "c128a" => Some(Self::Code128A),
            "c128b" => Some(Self::Code128B),
            "c128c" => Some(Self::Code128C),
            "upca" => Some(Self::UPCA),
            "ean13" => Some(Self::EAN13),
            "ean8" => Some(Self::EAN8),
            "msi" => Some(Self::MSI),
            "qr" => Some(Self::QR),
            "datamatrix" => Some(Self::DataMatrix),
            _ => None,
        }
    }

    pub fn to_word(&self) -> String {
        match self {
            BarcodeSymbology::Code128A => "c128a".to_string(),
            BarcodeSymbology::Code128B => "c128b".to_string(),
            BarcodeSymbology::Code128C => "c128c".to_string(),
            BarcodeSymbology::Code39 => "c39".to_string(),
            BarcodeSymbology::UPCA => "upca".to_string(),
            BarcodeSymbology::EAN13 => "ean13".to_string(),
            BarcodeSymbology::EAN8 => "ean8".to_string(),
            BarcodeSymbology::MSI => "msi".to_string(),
            BarcodeSymbology::QR => "qr".to_string(),
            BarcodeSymbology::DataMatrix => "datamatrix".to_string(),
        }
    }

    pub fn to_marker(&self) -> &str {
        match self {
            BarcodeSymbology::Code128A => "\u{00C0}",
            BarcodeSymbology::Code128B => "\u{0181}",
            BarcodeSymbology::Code128C => "\u{0106}",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub version: String,
    pub page: Page,
    pub slots: Vec<SlotDecl>,
    pub fonts: Vec<FontDecl>,
    pub assets: Vec<AssetDecl>,
    pub frames: Vec<FrameDecl>,
    pub draw_frames: Vec<FrameDrawBlock>,
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
    BarcodeSymbology(BarcodeSymbology),
}

impl StackValue {
    fn type_name(&self) -> &'static str {
        match self {
            StackValue::Number(_) => "number",
            StackValue::Text(_) => "string",
            StackValue::BarcodeSymbology(_) => "barcode",
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
    Number(NumberValue),
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
    SetCmyk {
        c: NumberValue,
        m: NumberValue,
        y: NumberValue,
        k: NumberValue,
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
    LoadImage {
        path: TextValue,
        alias: TextValue,
    },
    Barcode {
        value: TextValue,
        symbology: BarcodeSymbology,
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
    UnexpectedFrameToken {
        found: TokenKind,
        line: usize,
        column: usize,
    },
    UnknownFrameIndex {
        index: u32,
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
        let frames = self.parse_frames()?;
        let frame_lookup = Self::build_frame_lookup(&frames);
        let draw_frames = self.parse_draw(&slot_lookup, &frame_lookup)?;

        self.expect_eof()?;

        Ok(Template {
            version,
            page,
            slots,
            fonts,
            assets,
            frames,
            draw_frames,
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

    fn pop_barcode_symbology(
        stack: &mut Vec<StackValue>,
        operator: &str,
        token: &Token,
    ) -> Result<BarcodeSymbology, ParseError> {
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
            StackValue::BarcodeSymbology(symbol) => Ok(symbol),
            other => Err(ParseError::UnexpectedStackValue {
                operator: operator.to_string(),
                expected: "barcode".to_string(),
                found: other.type_name().to_string(),
                line: token.line,
                column: token.column,
            }),
        }
    }

    fn pop_string_or_number(
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
            StackValue::Text(text) => Ok(text),
            StackValue::Number(number) => Ok(TextValue::Number(number)),
            other => Err(ParseError::UnexpectedStackValue {
                operator: operator.to_string(),
                expected: "string or number".to_string(),
                found: other.type_name().to_string(),
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

    fn build_frame_lookup(frames: &[FrameDecl]) -> HashSet<u32> {
        frames.iter().map(|frame| frame.index).collect()
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

    fn parse_draw(
        &mut self,
        slots: &HashMap<String, SlotType>,
        declared_frames: &HashSet<u32>,
    ) -> Result<Vec<FrameDrawBlock>, ParseError> {
        self.expect_word("draw")?;
        self.expect_word("begin")?;

        let mut frames: Vec<FrameDrawBlock> = Vec::new();

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "draw block".to_string(),
                });
            }

            let frame = self.parse_frame_draw_block(slots, declared_frames)?;
            frames.push(frame);
        }

        self.expect_word("end")?;
        Ok(frames)
    }

    fn parse_frame_draw_block(
        &mut self,
        slots: &HashMap<String, SlotType>,
        declared_frames: &HashSet<u32>,
    ) -> Result<FrameDrawBlock, ParseError> {
        self.expect_word("frame")?;

        let idx_token = self.advance().clone();
        let index = match idx_token.kind {
            TokenKind::Number(n)
                if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= u32::MAX as f64 =>
            {
                n as u32
            }
            found => {
                return Err(ParseError::UnexpectedFrameToken {
                    found,
                    line: idx_token.line,
                    column: idx_token.column,
                });
            }
        };

        if !declared_frames.contains(&index) {
            return Err(ParseError::UnknownFrameIndex {
                index,
                line: idx_token.line,
                column: idx_token.column,
            });
        }

        self.expect_word("begin")?;

        let mut stack: Vec<StackValue> = Vec::new();
        let mut ops: Vec<DrawOp> = Vec::new();
        let mut current_path_kind: Option<CurrentPathKind> = None;

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "frame draw block".to_string(),
                });
            }

            if let Some(op) =
                self.parse_frame_draw_token(slots, &mut stack, &mut current_path_kind)?
            {
                ops.push(op);
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

        Ok(FrameDrawBlock { index, ops })
    }

    fn parse_frame_draw_token(
        &mut self,
        slots: &HashMap<String, SlotType>,
        stack: &mut Vec<StackValue>,
        current_path_kind: &mut Option<CurrentPathKind>,
    ) -> Result<Option<DrawOp>, ParseError> {
        let token = self.advance().clone();

        match &token.kind {
            TokenKind::Number(value) => {
                stack.push(StackValue::Number(NumberValue::Literal(*value)));
                Ok(None)
            }
            TokenKind::String(value) => {
                stack.push(StackValue::Text(TextValue::Literal(value.clone())));
                Ok(None)
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

                Ok(None)
            }
            TokenKind::Word(word) if word == "concat" => {
                Self::require_stack(stack, "concat", 1, &token)?;
                let count_value = Self::pop_number(stack, "concat", &token)?;
                let count = match count_value {
                    NumberValue::Literal(n) if n.is_finite() && n >= 0.0 && n.fract() == 0.0 => {
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

                Self::require_stack(stack, "concat", count, &token)?;
                let mut values: Vec<TextValue> = Vec::new();
                for _ in 0..count {
                    let value = Self::pop_string_or_number(stack, "concat", &token)?;
                    values.push(value);
                }
                values.reverse();
                stack.push(StackValue::Text(TextValue::Concat(values)));
                Ok(None)
            }
            TokenKind::Word(word) if word == "uppercase" => {
                Self::require_stack(stack, "uppercase", 1, &token)?;
                let value = Self::pop_string(stack, "uppercase", &token)?;
                stack.push(StackValue::Text(TextValue::UpperCase(Box::new(value))));
                Ok(None)
            }
            TokenKind::Word(word) if word == "lowercase" => {
                Self::require_stack(stack, "lowercase", 1, &token)?;
                let value = Self::pop_string(stack, "lowercase", &token)?;
                stack.push(StackValue::Text(TextValue::LowerCase(Box::new(value))));
                Ok(None)
            }
            TokenKind::Word(word) if word == "capitalize" => {
                Self::require_stack(stack, "capitalize", 1, &token)?;
                let value = Self::pop_string(stack, "capitalize", &token)?;
                stack.push(StackValue::Text(TextValue::Capitalize(Box::new(value))));
                Ok(None)
            }
            TokenKind::Word(word) if word == "titlecase" => {
                Self::require_stack(stack, "title", 1, &token)?;
                let value = Self::pop_string(stack, "title", &token)?;
                stack.push(StackValue::Text(TextValue::TitleCase(Box::new(value))));
                Ok(None)
            }
            TokenKind::Word(word) if ImageFit::from_word(word).is_some() => {
                let fit = ImageFit::from_word(word).unwrap();
                self.expect_word("imagefit")?;
                Ok(Some(DrawOp::SetImageFit { fit }))
            }
            TokenKind::Word(word) if word == "image" => {
                Self::require_stack(stack, "image", 5, &token)?;
                let height = Self::pop_number(stack, "image", &token)?;
                let width = Self::pop_number(stack, "image", &token)?;
                let y = Self::pop_number(stack, "image", &token)?;
                let x = Self::pop_number(stack, "image", &token)?;
                let asset = Self::pop_string(stack, "image", &token)?;

                Self::validate_literal_positive(&width, "image", "width", &token)?;
                Self::validate_literal_positive(&height, "image", "height", &token)?;

                Ok(Some(DrawOp::Image {
                    asset,
                    x,
                    y,
                    width,
                    height,
                }))
            }
            TokenKind::Word(word) if word == "loadimage" => {
                Self::require_stack(stack, "loadimage", 2, &token)?;
                let alias = Self::pop_string(stack, "loadimage", &token)?;
                let path = Self::pop_string(stack, "loadimage", &token)?;

                Ok(Some(DrawOp::LoadImage { path, alias }))
            }
            TokenKind::Word(word) if word == "font" => {
                Self::require_stack(stack, "font", 1, &token)?;
                let font = Self::pop_string(stack, "font", &token)?;
                Ok(Some(DrawOp::SetFontFamily { font }))
            }
            TokenKind::Word(word) if TextAlign::from_word(word).is_some() => {
                let align = TextAlign::from_word(word).unwrap();
                self.expect_word("align")?;
                Ok(Some(DrawOp::SetTextAlignment { align }))
            }
            TokenKind::Word(word) if VerticalAlign::from_word(word).is_some() => {
                let align = VerticalAlign::from_word(word).unwrap();
                self.expect_word("valign")?;
                Ok(Some(DrawOp::SetVerticalAlignment { align }))
            }
            TokenKind::Word(word) if LineBreakMode::from_word(word).is_some() => {
                let line_break = LineBreakMode::from_word(word).unwrap();
                self.expect_word("wrap")?;
                Ok(Some(DrawOp::SetLineBreakMode { line_break }))
            }
            TokenKind::Word(word) if TextFit::from_word(word).is_some() => {
                let fit = TextFit::from_word(word).unwrap();
                self.expect_word("textfit")?;
                Ok(Some(DrawOp::SetTextFit { fit }))
            }
            TokenKind::Word(word) if word == "textfitmin" => {
                Self::require_stack(stack, "textfitmin", 1, &token)?;
                let min = Self::pop_number(stack, "textfitmin", &token)?;
                Self::validate_literal_positive(&min, "textfitmin", "size", &token)?;
                Ok(Some(DrawOp::SetTextFitMinSize { min }))
            }
            TokenKind::Word(word) if word == "textfitmax" => {
                Self::require_stack(stack, "textfitmax", 1, &token)?;
                let max = Self::pop_number(stack, "textfitmax", &token)?;
                Self::validate_literal_positive(&max, "textfitmax", "size", &token)?;
                Ok(Some(DrawOp::SetTextFitMaxSize { max }))
            }
            TokenKind::Word(word) if word == "fontsize" => {
                Self::require_stack(stack, "fontsize", 1, &token)?;
                let size = Self::pop_number(stack, "fontsize", &token)?;
                Self::validate_literal_positive(&size, "fontsize", "size", &token)?;
                Ok(Some(DrawOp::SetFontSize { size }))
            }
            TokenKind::Word(word) if word == "textbox" => {
                Self::require_stack(stack, "textbox", 5, &token)?;
                let height = Self::pop_number(stack, "textbox", &token)?;
                let width = Self::pop_number(stack, "textbox", &token)?;
                let y = Self::pop_number(stack, "textbox", &token)?;
                let x = Self::pop_number(stack, "textbox", &token)?;
                let text = Self::pop_string(stack, "textbox", &token)?;

                Self::validate_literal_positive(&width, "textbox", "width", &token)?;
                Self::validate_literal_positive(&height, "textbox", "height", &token)?;

                Ok(Some(DrawOp::TextBox {
                    text,
                    x,
                    y,
                    width,
                    height,
                }))
            }
            TokenKind::Word(word) if word == "rgb" => {
                Self::require_stack(stack, "rgb", 3, &token)?;
                let b = Self::pop_number(stack, "rgb", &token)?;
                let g = Self::pop_number(stack, "rgb", &token)?;
                let r = Self::pop_number(stack, "rgb", &token)?;
                Ok(Some(DrawOp::SetRgb { r, g, b }))
            }
            TokenKind::Word(word) if word == "cmyk" => {
                Self::require_stack(stack, "cmyk", 4, &token)?;
                let k = Self::pop_number(stack, "cmyk", &token)?;
                let y = Self::pop_number(stack, "cmyk", &token)?;
                let m = Self::pop_number(stack, "cmyk", &token)?;
                let c = Self::pop_number(stack, "cmyk", &token)?;
                Ok(Some(DrawOp::SetCmyk { c, m, y, k }))
            }
            TokenKind::Word(word) if word == "strokewidth" => {
                Self::require_stack(stack, "strokewidth", 1, &token)?;
                let width = Self::pop_number(stack, "strokewidth", &token)?;
                Self::validate_literal_positive(&width, "strokewidth", "width", &token)?;
                Ok(Some(DrawOp::SetStrokeWidth { width }))
            }
            TokenKind::Word(word) if word == "line" => {
                if current_path_kind.is_some() {
                    return Err(ParseError::UnpaintedPath {
                        line: token.line,
                        column: token.column,
                    });
                }

                Self::require_stack(stack, "line", 4, &token)?;
                let y2 = Self::pop_number(stack, "line", &token)?;
                let x2 = Self::pop_number(stack, "line", &token)?;
                let y1 = Self::pop_number(stack, "line", &token)?;
                let x1 = Self::pop_number(stack, "line", &token)?;

                *current_path_kind = Some(CurrentPathKind::Line);
                Ok(Some(DrawOp::LinePath { x1, y1, x2, y2 }))
            }
            TokenKind::Word(word) if word == "rect" => {
                if current_path_kind.is_some() {
                    return Err(ParseError::UnpaintedPath {
                        line: token.line,
                        column: token.column,
                    });
                }

                Self::require_stack(stack, "rect", 4, &token)?;
                let height = Self::pop_number(stack, "rect", &token)?;
                let width = Self::pop_number(stack, "rect", &token)?;
                let y = Self::pop_number(stack, "rect", &token)?;
                let x = Self::pop_number(stack, "rect", &token)?;

                Self::validate_literal_positive(&width, "rect", "width", &token)?;
                Self::validate_literal_positive(&height, "rect", "height", &token)?;

                *current_path_kind = Some(CurrentPathKind::Rect);
                Ok(Some(DrawOp::RectPath {
                    x,
                    y,
                    width,
                    height,
                }))
            }
            TokenKind::Word(word) if word == "stroke" => {
                if current_path_kind.is_none() {
                    return Err(ParseError::NoCurrentPath {
                        operator: "stroke".to_string(),
                        line: token.line,
                        column: token.column,
                    });
                }

                *current_path_kind = None;
                Ok(Some(DrawOp::Stroke))
            }
            TokenKind::Word(word) if word == "fill" => match current_path_kind {
                Some(CurrentPathKind::Rect) => {
                    *current_path_kind = None;
                    Ok(Some(DrawOp::Fill))
                }
                Some(CurrentPathKind::Line) => Err(ParseError::CannotFillPath {
                    path: "line".to_string(),
                    line: token.line,
                    column: token.column,
                }),
                None => Err(ParseError::NoCurrentPath {
                    operator: "fill".to_string(),
                    line: token.line,
                    column: token.column,
                }),
            },
            TokenKind::Word(word) if BarcodeSymbology::from_word(word).is_some() => {
                let symbology = BarcodeSymbology::from_word(word).unwrap();
                stack.push(StackValue::BarcodeSymbology(symbology));
                Ok(None)
            }
            TokenKind::Word(word) if word == "barcode" => {
                Self::require_stack(stack, "barcode", 6, &token)?;
                let height = Self::pop_number(stack, "barcode", &token)?;
                let width = Self::pop_number(stack, "barcode", &token)?;
                let y = Self::pop_number(stack, "barcode", &token)?;
                let x = Self::pop_number(stack, "barcode", &token)?;
                let symbology = Self::pop_barcode_symbology(stack, "barcode", &token)?;
                let value = Self::pop_string(stack, "barcode", &token)?;

                Self::validate_literal_positive(&width, "barcode", "width", &token)?;
                Self::validate_literal_positive(&height, "barcode", "height", &token)?;

                Ok(Some(DrawOp::Barcode {
                    value,
                    symbology,
                    x,
                    y,
                    width,
                    height,
                }))
            }
            TokenKind::Comment(_) => Ok(None),
            found => Err(ParseError::UnexpectedDrawToken {
                found: found.clone(),
                line: token.line,
                column: token.column,
            }),
        }
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

    fn parse_frames(&mut self) -> Result<Vec<FrameDecl>, ParseError> {
        self.expect_word("frames")?;
        self.expect_word("begin")?;

        let mut frames: Vec<FrameDecl> = Vec::new();

        while !self.check_word("end") {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    context: "frame block".to_string(),
                });
            }
            frames.push(self.parse_frame_decl()?);
        }
        self.expect_word("end")?;
        Ok(frames)
    }

    fn parse_frame_decl(&mut self) -> Result<FrameDecl, ParseError> {
        let idx_token = self.advance().clone();
        let index = match idx_token.kind {
            TokenKind::Number(n)
                if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= u32::MAX as f64 =>
            {
                n as u32
            }
            found => {
                return Err(ParseError::UnexpectedFrameToken {
                    found: found.clone(),
                    line: idx_token.line,
                    column: idx_token.column,
                });
            }
        };

        let id_token = self.advance().clone();
        let id = match id_token.kind {
            TokenKind::Word(id) => id,
            found => {
                return Err(ParseError::UnexpectedFrameToken {
                    found: found.clone(),
                    line: id_token.line,
                    column: id_token.column,
                });
            }
        };

        Ok(FrameDecl { index, id })
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
