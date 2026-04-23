use png::{ColorType, Decoder};
use std::io::Cursor;
use tauri::image::Image;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-icon.png");

/// Decode the bundled tray icon and force every non-transparent pixel's RGB
/// channels to pure white while preserving alpha for menubar contrast.
pub fn load_white_masked_tray_icon() -> Result<Image<'static>, String> {
    let decoder = Decoder::new(Cursor::new(TRAY_ICON_BYTES));
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|error| error.to_string())?;

    if info.color_type != ColorType::Rgba {
        return Err(format!(
            "tray-icon.png must be RGBA; got {:?}",
            info.color_type
        ));
    }

    buf.truncate(info.buffer_size());

    for chunk in buf.chunks_exact_mut(4) {
        if chunk[3] == 0 {
            continue;
        }

        chunk[0] = 255;
        chunk[1] = 255;
        chunk[2] = 255;
    }

    Ok(Image::new_owned(buf, info.width, info.height))
}
