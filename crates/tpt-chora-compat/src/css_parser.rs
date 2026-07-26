pub struct CssParser {
    input: String,
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
            input,
            position: 0,
            line: 1,
        }
    }

    pub fn parse(&mut self) -> Result<ParsedCss, crate::CompatError> {
        let mut rules = Vec::new();

        while self.position < self.input.len() {
            self.skip_whitespace();
            if self.position >= self.input.len() {
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
        while self.position < self.input.len() && self.peek() != '}' {
            self.skip_whitespace();
            if self.peek() == '}' {
                break;
            }

            if let Some(decl) = self.parse_declaration()? {
                declarations.push(decl);
            }
            self.skip_whitespace();
        }

        if self.position < self.input.len() {
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
        if self.position < self.input.len() {
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

    fn read_until(&mut self, char: char) -> Result<String, crate::CompatError> {
        let start = self.position;
        while self.position < self.input.len() && self.peek() != char {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.position += 1;
        }
        Ok(self.input[start..self.position].to_string())
    }

    fn peek(&self) -> char {
        self.input
            .as_bytes()
            .get(self.position)
            .copied()
            .unwrap_or(b' ') as char
    }

    fn peek_next(&self) -> Option<char> {
        self.input
            .as_bytes()
            .get(self.position + 1)
            .copied()
            .map(|b| b as char)
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
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
        while self.position < self.input.len() - 1 {
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
        while self.position < self.input.len() && self.peek() != '{' && self.peek() != ';' {
            self.position += 1;
        }
        if self.position < self.input.len() && self.peek() == '{' {
            let mut depth = 1;
            self.position += 1;
            while self.position < self.input.len() && depth > 0 {
                match self.peek() {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                self.position += 1;
            }
        } else if self.position < self.input.len() {
            self.position += 1;
        }
    }
}
