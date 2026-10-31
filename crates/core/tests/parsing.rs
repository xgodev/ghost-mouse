//! Values typed by a person or an agent become the right types, or a clear error.

use ghost_mouse_core::{png_size, Button, Chord, Key, Modifiers, Point};

#[test]
fn point_reads_x_comma_y() {
    assert_eq!("454,494".parse::<Point>().unwrap(), Point { x: 454.0, y: 494.0 });
    assert_eq!(" 1.5 , 2 ".parse::<Point>().unwrap(), Point { x: 1.5, y: 2.0 });
}

#[test]
fn point_refuses_anything_else() {
    for bad in ["", "1", "a,b", "1,2,3", "NaN,1", "1,inf"] {
        assert!(bad.parse::<Point>().is_err(), "{bad} should be refused");
    }
}

#[test]
fn button_reads_its_name() {
    assert_eq!("left".parse::<Button>().unwrap(), Button::Left);
    assert_eq!("right".parse::<Button>().unwrap(), Button::Right);
    assert_eq!("middle".parse::<Button>().unwrap(), Button::Middle);
    assert!("thumb".parse::<Button>().is_err());
}

#[test]
fn chord_reads_a_plain_key() {
    let none = Modifiers::default();
    for (text, key) in [
        ("a", Key::Char('a')),
        ("A", Key::Char('a')),
        ("7", Key::Char('7')),
        ("return", Key::Return),
        ("enter", Key::Return),
        ("esc", Key::Escape),
        ("backspace", Key::Delete),
        ("f12", Key::F(12)),
        ("pagedown", Key::PageDown),
    ] {
        assert_eq!(text.parse::<Chord>().unwrap(), Chord { key, modifiers: none }, "{text}");
    }
}

#[test]
fn chord_reads_modifiers_in_any_order() {
    let chord: Chord = "cmd+shift+a".parse().unwrap();
    assert_eq!(chord.key, Key::Char('a'));
    assert!(chord.modifiers.command && chord.modifiers.shift);
    assert!(!chord.modifiers.option && !chord.modifiers.control);
    assert_eq!("shift+cmd+a".parse::<Chord>().unwrap(), chord);
    let chord: Chord = "ctrl+alt+left".parse().unwrap();
    assert_eq!(chord.key, Key::Left);
    assert!(chord.modifiers.control && chord.modifiers.option);
}

#[test]
fn chord_refuses_what_it_cannot_press() {
    for bad in ["", "cmd+", "cmd", "shift+cmd", "f13", "f0", "nokey", "hyper+a"] {
        assert!(bad.parse::<Chord>().is_err(), "{bad} should be refused");
    }
}

#[test]
fn png_size_reads_the_header() {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&1440u32.to_be_bytes());
    png.extend_from_slice(&900u32.to_be_bytes());
    assert_eq!(png_size(&png), Some((1440, 900)));
}

#[test]
fn png_size_refuses_other_bytes() {
    assert_eq!(png_size(b""), None);
    assert_eq!(png_size(b"\x89PNG\r\n\x1a\n"), None);
    assert_eq!(png_size(&[0u8; 32]), None);
}
