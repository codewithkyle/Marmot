use anyhow::Result;

use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub version: String,
    pub page: Page,
    pub slots: Vec<SlotDecl>,
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
    InvalidSlotType {
        value: String,
    },
    InvalidSlotRequirement {
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

        self.expect_eof()?;

        Ok(Template {
            version,
            page,
            slots,
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
                return true;
            }
            _ => {
                return false;
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
                line: 3,
                column: 1,
            }
        );
    }

    #[test]
    fn parses_template_without_slots() {
        let source = r#"%!PSL 0.1
page 612 792
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
}
