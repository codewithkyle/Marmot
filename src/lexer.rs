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

    fn is_word_char(ch: char) -> bool {
        return ch.is_ascii() && (ch.is_ascii_alphanumeric() || ch == '_');
    }

    fn consume_word(&mut self) -> String {
        let mut result = String::new();
        while let Some(ch) = self.peek() {
            if Self::is_word_char(ch) {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        return result;
    }

    fn consume_comment(&mut self) -> String {
        let mut result = String::new();
        self.advance(); // NOTE: consume '%'
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            result.push(ch);
            self.advance();
        }
        return result.trim().into();
    }

    fn consume_string(&mut self) -> Result<String, LexError> {
        let mut result = String::new();
        let line = self.line;
        let column = self.column;
        self.advance(); // NOTE: consume '('
        while let Some(ch) = self.peek() {
            match ch {
                '\n' => {
                    return Err(LexError::UnterminatedString { line, column });
                }
                '\\' => {
                    self.advance(); // NOTE: consume escape '\'
                    let escaped = match self.peek() {
                        Some('(') => '(',
                        Some(')') => ')',
                        Some('\\') => '\\',
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some(other) => other,
                        None => {
                            return Err(LexError::UnterminatedString { line, column });
                        }
                    };
                    result.push(escaped);
                    self.advance(); // NOTE: consume the escaped char
                }
                ')' => {
                    self.advance();
                    return Ok(result);
                }
                _ => {
                    result.push(ch);
                    self.advance();
                }
            }
        }
        Err(LexError::UnterminatedString { line, column })
    }

    fn consume_number(&mut self) -> Result<f64, LexError> {
        let mut result = String::new();
        let mut has_decimal = false;
        let line = self.line;
        let column = self.column;
        while let Some(ch) = self.peek() {
            match ch {
                c if c.is_whitespace() || !c.is_ascii() => {
                    break;
                }
                c if c.is_ascii_digit() => {
                    result.push(ch);
                    self.advance();
                }
                c if c == '.' => {
                    result.push(ch);
                    if has_decimal {
                        return Err(LexError::InvalidNumber {
                            value: result,
                            line,
                            column,
                        });
                    }
                    has_decimal = true;
                    self.advance();
                }
                c if !c.is_ascii_digit() => {
                    result.push(ch);
                    return Err(LexError::InvalidNumber {
                        value: result,
                        line,
                        column,
                    });
                }
                _ => {
                    return Err(LexError::UnknownCharacter { ch, line, column });
                }
            }
        }
        let value = result.parse::<f64>().map_err(|_| LexError::InvalidNumber {
            value: result.clone(),
            line: line,
            column: column,
        })?;
        Ok(value)
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
                c if c.is_ascii_digit() => match self.consume_number() {
                    Ok(value) => {
                        tokens.push(Token {
                            kind: TokenKind::Number(value),
                            line,
                            column,
                        });
                    }
                    Err(err) => return Err(err),
                },
                '%' => {
                    let value = self.consume_comment();
                    tokens.push(Token {
                        kind: TokenKind::Comment(value.to_string()),
                        line,
                        column,
                    });
                }
                '(' => match self.consume_string() {
                    Ok(value) => {
                        tokens.push(Token {
                            kind: TokenKind::String(value.to_string()),
                            line,
                            column,
                        });
                    }
                    Err(err) => return Err(err),
                },
                _c if Self::is_word_char(ch) => {
                    let value = self.consume_word();
                    tokens.push(Token {
                        kind: TokenKind::Word(value.to_string()),
                        line,
                        column,
                    });
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
}
