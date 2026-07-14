use image::{ImageFormat, ImageBuffer, Rgb, load_from_memory, GenericImageView};
use std::io::Cursor;

/// Charge une image et retourne (pixels RVB, largeur, hauteur, format).
pub fn load_image(data: &[u8]) -> Option<(Vec<u8>, u32, u32, ImageFormat)> {
    let format = image::guess_format(data).ok()?;
    let img = load_from_memory(data).ok()?;
    let (w, h) = img.dimensions();
    let rgb = img.to_rgb8();
    Some((rgb.into_raw(), w, h, format))
}

/// Sauvegarde une image dans le format donné.
pub fn save_image(pixels: &[u8], width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    // Spécifier explicitement le type de pixel
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, pixels.to_vec())
        .expect("Dimensions invalides");
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, format).expect("Erreur d'écriture");
    cursor.into_inner()
}