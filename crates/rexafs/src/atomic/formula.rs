//! Deliberately limited stoichiometric grammar with original byte positions.
use super::*;
pub(super) fn parse(input: &str, db: &XrayDb) -> Result<BTreeMap<String, f64>> {
    if input.is_empty() || input.len() > 512 {
        return Err(invalid("formula needs 1..512 ASCII bytes"));
    }
    let mut parser = Parser { input, pos: 0, db };
    let result = parser.sequence(0)?;
    if parser.pos != input.len() {
        return Err(parser.error("unexpected closing parenthesis"));
    }
    Ok(result)
}
struct Parser<'a> {
    input: &'a str,
    pos: usize,
    db: &'a XrayDb,
}
impl Parser<'_> {
    fn error(&self, reason: &str) -> AtomicDataError {
        invalid(format!("formula byte {}: {reason}", self.pos))
    }
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }
    fn sequence(&mut self, depth: usize) -> Result<BTreeMap<String, f64>> {
        if depth > 32 {
            return Err(self.error("parentheses exceed depth 32"));
        }
        let mut atoms = BTreeMap::new();
        while let Some(ch) = self.peek() {
            if ch == b')' {
                break;
            }
            let term = if ch == b'(' {
                self.pos += 1;
                let term = self.sequence(depth + 1)?;
                if self.peek() != Some(b')') {
                    return Err(self.error("missing closing parenthesis"));
                }
                self.pos += 1;
                term
            } else if ch.is_ascii_uppercase() {
                let start = self.pos;
                self.pos += 1;
                if self.peek().is_some_and(|c| c.is_ascii_lowercase()) {
                    self.pos += 1;
                }
                let symbol = &self.input[start..self.pos];
                if self.db.symbol(symbol).ok() != Some(symbol) {
                    return Err(invalid(format!(
                        "formula byte {start}: unsupported element or isotope {symbol}"
                    )));
                }
                BTreeMap::from([(symbol.to_owned(), 1.)])
            } else {
                return Err(self.error("expected an element symbol or '('; charge, isotope, hydrate-dot and named-material syntax are unsupported"));
            };
            let factor = self.count()?;
            for (symbol, n) in term {
                let value = atoms.entry(symbol).or_insert(0.);
                *value += n * factor;
                if !value.is_finite() || *value <= 0. {
                    return Err(self.error("atom counts must be finite and positive"));
                }
            }
        }
        if atoms.is_empty() {
            return Err(self.error("empty formula/group"));
        }
        Ok(atoms)
    }
    fn count(&mut self) -> Result<f64> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if start == self.pos {
            return Ok(1.);
        }
        let value = self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|_| self.error("invalid decimal count"))?;
        if !value.is_finite() || value <= 0. {
            return Err(self.error("counts must be finite and positive"));
        }
        Ok(value)
    }
}
