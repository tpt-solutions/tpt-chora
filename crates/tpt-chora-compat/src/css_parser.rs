pub struct CssParser {
    chars: Vec<char>,
    position: usize,
    line: u32,
}

#[derive(Debug, Clone)]
pub struct ParsedCss {
    pub rules: Vec<CssRule>,
}

#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub declarations: Vec<CssDeclaration>,
}

#[derive(Debug, Clone)]
pub struct CssDeclaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}

impl CssParser {
    pub fn new(input: String) -> Self {
        Self {
            chars: input.chars().collect(),
            position: 0,
            line: 1,
        }
    }

    pub fn parse(&mut self) -> Result<ParsedCss, crate::CompatError> {
        let mut rules = Vec::new();

        while self.position < self.chars.len() {
            self.skip_whitespace();
            if self.position >= self.chars.len() {
                break;
            }

            if self.peek() == '/' && self.peek_next() == Some('*') {
                self.skip_comment();
                continue;
            }

            if self.peek() == '@' {
                self.skip_at_rule();
                continue;
            }

            if let Some(rule) = self.parse_rule()? {
                rules.push(rule);
            }
        }

        Ok(ParsedCss { rules })
    }

    fn parse_rule(&mut self) -> Result<Option<CssRule>, crate::CompatError> {
        let selector = self.read_until('{')?;
        self.position += 1;

        let mut declarations = Vec::new();
        while self.position < self.chars.len() && self.peek() != '}' {
            self.skip_whitespace();
            if self.peek() == '}' {
                break;
            }

            if let Some(decl) = self.parse_declaration()? {
                declarations.push(decl);
            }
            self.skip_whitespace();
        }

        if self.position < self.chars.len() {
            self.position += 1;
        }

        Ok(Some(CssRule {
            selector: selector.trim().to_string(),
            declarations,
        }))
    }

    fn parse_declaration(&mut self) -> Result<Option<CssDeclaration>, crate::CompatError> {
        let property = self.read_until(':')?.trim().to_string();
        self.position += 1;
        self.skip_whitespace();

        let value = self.read_until(';')?.trim().to_string();
        if self.position < self.chars.len() {
            self.position += 1;
        }

        let important = value.ends_with("!important");
        let value = if important {
            value.trim_end_matches("!important").trim().to_string()
        } else {
            value
        };

        if property.is_empty() || value.is_empty() {
            return Ok(None);
        }

        Ok(Some(CssDeclaration {
            property,
            value,
            important,
        }))
    }

    fn read_until(&mut self, target: char) -> Result<String, crate::CompatError> {
        let start = self.position;
        while self.position < self.chars.len() && self.peek() != target {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.position += 1;
        }
        Ok(self.chars[start..self.position].iter().collect())
    }

    fn peek(&self) -> char {
        self.chars.get(self.position).copied().unwrap_or(' ')
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.position + 1).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.chars.len() {
            match self.peek() {
                ' ' | '\t' | '\r' | '\n' => {
                    if self.peek() == '\n' {
                        self.line += 1;
                    }
                    self.position += 1;
                }
                _ => break,
            }
        }
    }

    fn skip_comment(&mut self) {
        self.position += 2;
        while self.position + 1 < self.chars.len() {
            if self.peek() == '*' && self.peek_next() == Some('/') {
                self.position += 2;
                return;
            }
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.position += 1;
        }
    }

    fn skip_at_rule(&mut self) {
        while self.position < self.chars.len() && self.peek() != '{' && self.peek() != ';' {
            self.position += 1;
        }
        if self.position < self.chars.len() && self.peek() == '{' {
            let mut depth = 1;
            self.position += 1;
            while self.position < self.chars.len() && depth > 0 {
                match self.peek() {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                self.position += 1;
            }
        } else if self.position < self.chars.len() {
            self.position += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ascii_rules() {
        let mut parser = CssParser::new(".btn { color: red; }".into());
        let css = parser.parse().unwrap();
        assert_eq!(css.rules.len(), 1);
        assert_eq!(css.rules[0].selector, ".btn");
        assert_eq!(css.rules[0].declarations[0].property, "color");
        assert_eq!(css.rules[0].declarations[0].value, "red");
    }

    #[test]
    fn parse_non_ascii_input() {
        let mut parser = CssParser::new(".btn { color: r\u{00E9}d; }".into());
        let css = parser.parse().unwrap();
        assert_eq!(css.rules.len(), 1);
        assert_eq!(css.rules[0].declarations[0].value, "r\u{00E9}d");
    }

    #[test]
    fn parse_cjk_selectors() {
        let mut parser = CssParser::new("\u{4E00}\u{5185}\u{5BB9} { font-size: 16px; }".into());
        let css = parser.parse().unwrap();
        assert_eq!(css.rules.len(), 1);
        assert_eq!(css.rules[0].selector, "\u{4E00}\u{5185}\u{5BB9}");
    }

    #[test]
    fn parse_emoji_in_value() {
        let mut parser = CssParser::new(".icon { content: \"\u{1F600}\"; }".into());
        let css = parser.parse().unwrap();
        assert_eq!(css.rules[0].declarations[0].value, "\"\u{1F600}\"");
    }
}
