//! Maps keys to macOS virtual key numbers (US keyboard positions).

use ghost_mouse_core::Key;

/// Returns the virtual key number, or `None` for a key the US keyboard does not have.
pub(crate) fn virtual_key(key: Key) -> Option<u16> {
    const CHARS: &[(char, u16)] = &[
        ('a', 0),
        ('s', 1),
        ('d', 2),
        ('f', 3),
        ('h', 4),
        ('g', 5),
        ('z', 6),
        ('x', 7),
        ('c', 8),
        ('v', 9),
        ('b', 11),
        ('q', 12),
        ('w', 13),
        ('e', 14),
        ('r', 15),
        ('y', 16),
        ('t', 17),
        ('1', 18),
        ('2', 19),
        ('3', 20),
        ('4', 21),
        ('6', 22),
        ('5', 23),
        ('=', 24),
        ('9', 25),
        ('7', 26),
        ('-', 27),
        ('8', 28),
        ('0', 29),
        (']', 30),
        ('o', 31),
        ('u', 32),
        ('[', 33),
        ('i', 34),
        ('p', 35),
        ('l', 37),
        ('j', 38),
        ('\'', 39),
        ('k', 40),
        (';', 41),
        ('\\', 42),
        (',', 43),
        ('/', 44),
        ('n', 45),
        ('m', 46),
        ('.', 47),
        ('`', 50),
    ];
    const FUNCTIONS: [u16; 12] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111];
    match key {
        Key::Char(ch) => CHARS.iter().find(|(c, _)| *c == ch).map(|(_, code)| *code),
        Key::F(number) => FUNCTIONS.get(usize::from(number).checked_sub(1)?).copied(),
        Key::Return => Some(36),
        Key::Tab => Some(48),
        Key::Space => Some(49),
        Key::Delete => Some(51),
        Key::Escape => Some(53),
        Key::Home => Some(115),
        Key::PageUp => Some(116),
        Key::ForwardDelete => Some(117),
        Key::End => Some(119),
        Key::PageDown => Some(121),
        Key::Left => Some(123),
        Key::Right => Some(124),
        Key::Down => Some(125),
        Key::Up => Some(126),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_have_their_number() {
        assert_eq!(virtual_key(Key::Char('a')), Some(0));
        assert_eq!(virtual_key(Key::Char('0')), Some(29));
        assert_eq!(virtual_key(Key::Return), Some(36));
        assert_eq!(virtual_key(Key::Escape), Some(53));
        assert_eq!(virtual_key(Key::Up), Some(126));
        assert_eq!(virtual_key(Key::F(1)), Some(122));
        assert_eq!(virtual_key(Key::F(12)), Some(111));
    }

    #[test]
    fn unknown_keys_have_none() {
        assert_eq!(virtual_key(Key::Char('ç')), None);
        assert_eq!(virtual_key(Key::F(13)), None);
    }
}
