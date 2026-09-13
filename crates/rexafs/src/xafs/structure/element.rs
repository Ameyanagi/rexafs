//! Element table (Z = 1…103): symbol, name, mass, radii and CPK colours.

use super::element_table::ELEMENTS;

/// Static element data. Radii in Å, mass in u, colour as RGB (Jmol CPK).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    /// Atomic number; the bundled table spans 1 through 103.
    pub z: u8,
    /// Standard chemical symbol, such as Cu.
    pub symbol: &'static str,
    /// English element name stored in the bundled table.
    pub name: &'static str,
    /// Tabulated atomic mass in unified atomic mass units (u), not an isotope-specific fit value.
    pub mass: f64,
    /// Tabulated covalent radius in Å, used for structure display heuristics.
    pub covalent_radius: f64,
    /// Tabulated atomic radius in Å; not an experimentally inferred bond distance.
    pub atomic_radius: f64,
    /// Jmol/CPK display color as red, green, blue bytes from 0 to 255.
    pub cpk: [u8; 3],
}

impl Element {
    /// All elements, ordered by Z.
    pub fn all() -> &'static [Element] {
        &ELEMENTS
    }

    /// Look up by atomic number.
    pub fn from_z(z: u8) -> Option<&'static Element> {
        if z == 0 {
            return None;
        }
        ELEMENTS.get(z as usize - 1)
    }

    /// Look up by exact symbol (case-insensitive).
    pub fn from_symbol(symbol: &str) -> Option<&'static Element> {
        let symbol = symbol.trim();
        ELEMENTS
            .iter()
            .find(|e| e.symbol.eq_ignore_ascii_case(symbol))
    }

    /// Look up from a CIF-style label or type symbol: `Fe2+`, `O1`, `RuA`,
    /// `Ca2`, `Wat` (→ W), `D` or `T` (→ H). Tries the leading two letters first
    /// (`Ru` in `RuA`), then one (`O` in `O1`), so `Co1` is cobalt while
    /// `CO` … is also cobalt — CIF labels are element-first by convention.
    /// This heuristic is not strict chemical-name validation; prefer an explicit
    /// element symbol when labels contain descriptive words or isotope information.
    pub fn from_label(label: &str) -> Option<&'static Element> {
        let letters: String = label
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        if letters.is_empty() {
            return None;
        }
        if letters.eq_ignore_ascii_case("D") || letters.eq_ignore_ascii_case("T") {
            return Element::from_symbol("H");
        }
        let mut chars = letters.chars();
        let first = chars.next()?.to_ascii_uppercase();
        let second = chars.next().map(|c| c.to_ascii_lowercase());
        if let Some(second) = second {
            let two = format!("{first}{second}");
            if let Some(e) = Element::from_symbol(&two) {
                return Some(e);
            }
        }
        Element::from_symbol(&first.to_string())
    }

    /// Colour as 0–1 floats, convenient for plotting APIs.
    pub fn cpk_f32(&self) -> [f32; 3] {
        [
            self.cpk[0] as f32 / 255.0,
            self.cpk[1] as f32 / 255.0,
            self.cpk[2] as f32 / 255.0,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_resolve_to_elements() {
        assert_eq!(Element::from_label("Ru").unwrap().z, 44);
        assert_eq!(Element::from_label("RuA").unwrap().z, 44);
        assert_eq!(Element::from_label("Fe2+").unwrap().symbol, "Fe");
        assert_eq!(Element::from_label("O1").unwrap().symbol, "O");
        assert_eq!(Element::from_label("Co1").unwrap().symbol, "Co");
        assert_eq!(Element::from_label("D").unwrap().symbol, "H");
        assert!(Element::from_label("Xx").is_none());
        assert!(Element::from_label("").is_none());
        assert!(Element::from_label("1").is_none());
        assert_eq!(Element::from_z(103).unwrap().symbol, "Lr");
        assert!(Element::from_z(0).is_none());
    }
}
