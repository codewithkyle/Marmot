use std::{iter::Peekable, str::Chars};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Word(String),
    Number(f64),
    String(String),
    Comment(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnknownCharacter {
        ch: char,
        line: usize,
        column: usize,
    },
    UnterminatedString {
        line: usize,
        column: usize,
    },
    InvalidNumber {
        value: String,
        line: usize,
        column: usize,
    },
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next()?;

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(ch)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            let line = self.line;
            let column = self.column;

            match ch {
                c if c.is_whitespace() => {
                    self.advance();
                }
                c if c.is_ascii_alphabetic() => {
                    tokens.push(Token {
                        kind: TokenKind::Word(c.to_string()),
                        line,
                        column,
                    });
                    self.advance();
                }
                c if c.is_ascii_digit() => {
                    tokens.push(Token {
                        kind: TokenKind::Number(c.to_digit(10).unwrap() as f64),
                        line,
                        column,
                    });
                    self.advance();
                }
                _ => {
                    return Err(LexError::UnknownCharacter { ch, line, column });
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            column: self.column,
        });

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
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
}
