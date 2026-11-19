// src/text.rs
// Utilidades de texto compartidas.

/// Recorta `s` a como mucho `max_bytes` bytes sin partir un carácter UTF-8.
///
/// `&s[..n]` hace panic si `n` cae en medio de un carácter multibyte (á, ñ,
/// —, →, emojis), algo frecuente en respuestas en español: aquí se retrocede
/// hasta el límite de carácter anterior.
pub fn truncate(s: &str, max_bytes: usize) -> &str {
    &s[..s.floor_char_boundary(max_bytes)]
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn no_parte_caracteres_multibyte() {
        // "á" ocupa los bytes 1..3: cortar en 2 debe retroceder a 1.
        assert_eq!(truncate("bárbaro", 2), "b");
        assert_eq!(truncate("a→b", 2), "a");
    }

    #[test]
    fn texto_corto_o_limite_exacto_queda_igual() {
        assert_eq!(truncate("hola", 10), "hola");
        assert_eq!(truncate("hola", 4), "hola");
        assert_eq!(truncate("", 5), "");
    }
}
