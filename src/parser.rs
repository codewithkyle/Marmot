use anyhow::Result;

use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub version: String,
    pub page: Page,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
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

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn is_eof(&self) -> bool {
        matches!(self.tokens[self.current].kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if self.is_eof() {
            return &self.tokens[self.current]
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

    pub fn parse_template(&mut self) -> Result<Template, ParseError> {
        let version = self.parse_header()?;
        let page = self.parse_page()?;

        self.expect_eof()?;

        Ok(Template { version, page })
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

        assert_eq!(
            err,
            ParseError::ExpectedHeader {
                line: 1,
                column: 1,
            }
        );
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
}
