use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchValue {
    Exact(String),
    Prefix(String),
    TimeRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Term {
        field: Option<String>,
        value: MatchValue,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Word(String),
    Quoted(String),
    Colon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    And,
    Or,
    Not,
    To,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    at: usize,
}

#[derive(Debug, Clone, Error, PartialEq, Eq, Serialize, Deserialize)]
#[error("query error at character {position}: {message}")]
pub struct ParseError {
    pub position: usize,
    pub message: String,
}

fn lex(input: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < chars.len() {
        let (at, ch) = chars[cursor];
        if ch.is_whitespace() {
            cursor += 1;
            continue;
        }
        let simple = match ch {
            ':' => Some(TokenKind::Colon),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            _ => None,
        };
        if let Some(kind) = simple {
            tokens.push(Token { kind, at });
            cursor += 1;
            continue;
        }
        if ch == '"' {
            cursor += 1;
            let mut value = String::new();
            let mut closed = false;
            while cursor < chars.len() {
                let (_, current) = chars[cursor];
                cursor += 1;
                if current == '"' {
                    closed = true;
                    break;
                }
                if current == '\\' && cursor < chars.len() {
                    value.push(chars[cursor].1);
                    cursor += 1;
                } else {
                    value.push(current);
                }
            }
            if !closed {
                return Err(ParseError {
                    position: at,
                    message: "unterminated quoted value".into(),
                });
            }
            tokens.push(Token {
                kind: TokenKind::Quoted(value),
                at,
            });
            continue;
        }

        let start = cursor;
        while cursor < chars.len() {
            let ch = chars[cursor].1;
            if ch.is_whitespace() || matches!(ch, ':' | '(' | ')' | '[' | ']') {
                break;
            }
            cursor += 1;
        }
        let start_byte = chars[start].0;
        let end_byte = chars.get(cursor).map(|item| item.0).unwrap_or(input.len());
        let word = &input[start_byte..end_byte];
        let kind = match word.to_ascii_uppercase().as_str() {
            "AND" => TokenKind::And,
            "OR" => TokenKind::Or,
            "NOT" => TokenKind::Not,
            "TO" => TokenKind::To,
            _ => TokenKind::Word(word.to_owned()),
        };
        tokens.push(Token { kind, at });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        at: input.len(),
    });
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;
        while self.current().kind == TokenKind::Or {
            self.advance();
            expr = Expr::Or(Box::new(expr), Box::new(self.parse_and()?));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            if self.current().kind == TokenKind::And {
                self.advance();
            } else if !matches!(
                self.current().kind,
                TokenKind::Word(_) | TokenKind::Quoted(_) | TokenKind::Not | TokenKind::LParen
            ) {
                break;
            }
            expr = Expr::And(Box::new(expr), Box::new(self.parse_unary()?));
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.current().kind == TokenKind::Not {
            self.advance();
            return Ok(Expr::Not(Box::new(self.parse_unary()?)));
        }
        if self.current().kind == TokenKind::LParen {
            self.advance();
            let expr = self.parse_or()?;
            if self.current().kind != TokenKind::RParen {
                return Err(self.error("expected ')'"));
            }
            self.advance();
            return Ok(expr);
        }
        self.parse_term()
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance();
        let raw = match token.kind {
            TokenKind::Word(value) | TokenKind::Quoted(value) => value,
            _ => {
                return Err(ParseError {
                    position: token.at,
                    message: "expected a search term".into(),
                });
            }
        };

        if self.current().kind != TokenKind::Colon {
            return Ok(Expr::Term {
                field: None,
                value: to_match_value(raw),
            });
        }

        self.advance();
        let field = raw.to_ascii_lowercase();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            return self.parse_range(field);
        }
        let value_token = self.advance();
        let value = match value_token.kind {
            TokenKind::Word(value) | TokenKind::Quoted(value) => value,
            _ => {
                return Err(ParseError {
                    position: value_token.at,
                    message: "expected a field value".into(),
                });
            }
        };
        Ok(Expr::Term {
            field: Some(field),
            value: to_match_value(value),
        })
    }

    fn parse_range(&mut self, field: String) -> Result<Expr, ParseError> {
        if field != "timestamp" && field != "time" {
            return Err(self.error("ranges are supported only for timestamp"));
        }
        let mut left = String::new();
        let mut right = String::new();
        let mut after_to = false;
        while !matches!(self.current().kind, TokenKind::RBracket | TokenKind::Eof) {
            let token = self.advance();
            match token.kind {
                TokenKind::To if !after_to => after_to = true,
                TokenKind::Colon => {
                    if after_to { &mut right } else { &mut left }.push(':');
                }
                TokenKind::Word(value) | TokenKind::Quoted(value) => {
                    if after_to { &mut right } else { &mut left }.push_str(&value);
                }
                _ => return Err(self.error("invalid timestamp range")),
            }
        }
        if self.current().kind != TokenKind::RBracket || !after_to {
            return Err(self.error("expected 'TO' and closing ']' in range"));
        }
        self.advance();
        let start = DateTime::parse_from_rfc3339(&left)
            .map_err(|_| self.error("invalid RFC 3339 range start"))?
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339(&right)
            .map_err(|_| self.error("invalid RFC 3339 range end"))?
            .with_timezone(&Utc);
        if start > end {
            return Err(self.error("range start is after range end"));
        }
        Ok(Expr::Term {
            field: Some("timestamp".into()),
            value: MatchValue::TimeRange { start, end },
        })
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            position: self.current().at,
            message: message.into(),
        }
    }
}

fn to_match_value(value: String) -> MatchValue {
    if let Some(prefix) = value.strip_suffix('*') {
        MatchValue::Prefix(prefix.to_ascii_lowercase())
    } else {
        MatchValue::Exact(value.to_ascii_lowercase())
    }
}

pub fn parse_query(input: &str) -> Result<Expr, ParseError> {
    if input.trim().is_empty() {
        return Err(ParseError {
            position: 0,
            message: "query cannot be empty".into(),
        });
    }
    let mut parser = Parser {
        tokens: lex(input)?,
        cursor: 0,
    };
    let expr = parser.parse_or()?;
    if parser.current().kind != TokenKind::Eof {
        return Err(parser.error("unexpected token"));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn and_binds_more_tightly_than_or() {
        let parsed = parse_query("user:ana OR host:dc01 AND NOT outcome:success").unwrap();
        assert!(matches!(parsed, Expr::Or(_, _)));
        let Expr::Or(_, right) = parsed else {
            unreachable!()
        };
        assert!(matches!(*right, Expr::And(_, _)));
    }

    #[test]
    fn supports_quoted_values_and_implicit_and() {
        let parsed = parse_query("message:\"invalid password\" severity:high").unwrap();
        assert!(matches!(parsed, Expr::And(_, _)));
    }

    #[test]
    fn rejects_unclosed_groups() {
        assert!(parse_query("(user:ana OR host:dc01").is_err());
    }

    #[test]
    fn parses_rfc3339_ranges() {
        let parsed =
            parse_query("timestamp:[2026-01-01T00:00:00Z TO 2026-01-01T01:00:00Z]").unwrap();
        assert!(matches!(
            parsed,
            Expr::Term {
                value: MatchValue::TimeRange { .. },
                ..
            }
        ));
    }
}
