//! Reads a picture's size from its PNG header.

/// Returns width and height in pixels, or `None` when the bytes are not a PNG.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    let rest = bytes.strip_prefix(SIGNATURE)?;
    let width = rest.get(..4)?.try_into().ok().map(u32::from_be_bytes)?;
    let height = rest.get(4..8)?.try_into().ok().map(u32::from_be_bytes)?;
    Some((width, height))
}
