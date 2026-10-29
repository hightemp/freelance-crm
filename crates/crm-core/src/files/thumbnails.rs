//! Image thumbnails (FR-FI-3): PNG, at most 320 px on the longer side,
//! turned upright according to the EXIF orientation.

use std::io::Cursor;

/// Longer side of a thumbnail, in pixels.
pub const MAX_SIDE: u32 = 320;

/// A PNG thumbnail, or `None` when the data is not a supported image.
pub fn thumbnail(data: &[u8]) -> Option<Vec<u8>> {
    use image::ImageDecoder;

    let mut decoder = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    let orientation = decoder.orientation().ok()?;
    let mut image = image::DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    if image.width() > MAX_SIDE || image.height() > MAX_SIDE {
        image = image.thumbnail(MAX_SIDE, MAX_SIDE);
    }
    let mut out = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::{DynamicImage, ImageEncoder, ImageFormat, RgbImage};

    fn encoded(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(RgbImage::new(width, height))
            .write_to(&mut Cursor::new(&mut out), format)
            .unwrap();
        out
    }

    /// A PNG of the given size, for other modules' tests.
    pub(crate) fn png(width: u32, height: u32) -> Vec<u8> {
        encoded(width, height, ImageFormat::Png)
    }

    fn size_of(png: &[u8]) -> (u32, u32) {
        let image = image::load_from_memory_with_format(png, ImageFormat::Png).unwrap();
        (image.width(), image.height())
    }

    #[test]
    fn png_jpeg_and_webp_shrink_keeping_proportions() {
        assert_eq!(size_of(&thumbnail(&png(640, 480)).unwrap()), (320, 240));
        assert_eq!(
            size_of(&thumbnail(&encoded(100, 1000, ImageFormat::Jpeg)).unwrap()),
            (32, 320)
        );
        assert_eq!(
            size_of(&thumbnail(&encoded(800, 400, ImageFormat::WebP)).unwrap()),
            (320, 160)
        );
    }

    #[test]
    fn small_images_are_not_enlarged() {
        assert_eq!(size_of(&thumbnail(&png(200, 100)).unwrap()), (200, 100));
    }

    /// TIFF-encoded EXIF with a single Orientation tag.
    fn exif_orientation(value: u16) -> Vec<u8> {
        let mut exif = b"II*\0\x08\0\0\0".to_vec();
        exif.extend_from_slice(&1u16.to_le_bytes()); // one entry
        exif.extend_from_slice(&0x0112u16.to_le_bytes()); // Orientation
        exif.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        exif.extend_from_slice(&1u32.to_le_bytes()); // count
        exif.extend_from_slice(&value.to_le_bytes());
        exif.extend_from_slice(&[0, 0]); // padding
        exif.extend_from_slice(&0u32.to_le_bytes()); // no next IFD
        exif
    }

    #[test]
    fn exif_orientation_is_applied() {
        let mut jpeg = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut jpeg);
        encoder.set_exif_metadata(exif_orientation(6)).unwrap();
        let image = RgbImage::new(40, 20);
        encoder
            .write_image(image.as_raw(), 40, 20, image::ExtendedColorType::Rgb8)
            .unwrap();
        // Rotated by 90°: the portrait the camera was held as.
        assert_eq!(size_of(&thumbnail(&jpeg).unwrap()), (20, 40));
    }

    #[test]
    fn other_data_has_no_thumbnail() {
        assert_eq!(thumbnail(b"%PDF-1.7 not an image"), None);
        assert_eq!(thumbnail(b""), None);
        // A broken image is not an error either.
        assert_eq!(thumbnail(&png(10, 10)[..20]), None);
    }
}
