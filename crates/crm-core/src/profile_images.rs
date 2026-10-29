//! Images of the freelancer's profile for documents (FR-FN-5): a logo, a
//! signature and a stamp. PNG or JPEG up to 2 MB, kept in the database so
//! that backups carry them.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::domain::Clock;
use crate::{CoreError, CoreResult};

pub const MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    Logo,
    Signature,
    Stamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// `image/png` or `image/jpeg`.
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

pub struct ProfileImages<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ProfileImages<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    pub fn set(&self, kind: ImageKind, bytes: Vec<u8>) -> CoreResult<()> {
        if bytes.len() > MAX_BYTES {
            return Err(CoreError::new("file.too_large").with("max", MAX_BYTES));
        }
        let mime = mime_of(&bytes).ok_or_else(|| CoreError::new("image.unsupported"))?;
        // A broken image would break every invoice PDF later.
        if image::load_from_memory(&bytes).is_err() {
            return Err(CoreError::new("image.unsupported"));
        }
        self.conn.execute(
            "INSERT INTO profile_images (kind, mime, data, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (kind) DO UPDATE SET
                 mime = excluded.mime, data = excluded.data, updated_at = excluded.updated_at",
            rusqlite::params![kind, mime, bytes, self.clock.now()],
        )?;
        Ok(())
    }

    pub fn get(&self, kind: ImageKind) -> CoreResult<Option<Image>> {
        let row: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM profile_images WHERE kind = ?1",
                [kind],
                |r| r.get(0),
            )
            .optional()?;
        Ok(row.and_then(|bytes| mime_of(&bytes).map(|mime| Image { mime, bytes })))
    }

    pub fn delete(&self, kind: ImageKind) -> CoreResult<()> {
        self.conn
            .execute("DELETE FROM profile_images WHERE kind = ?1", [kind])?;
        Ok(())
    }
}

crate::db::text_enum!(ImageKind { Logo => "logo", Signature => "signature", Stamp => "stamp" });

/// The type of an image by its signature bytes.
fn mime_of(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};

    fn png() -> Vec<u8> {
        crate::files::thumbnails::tests::png(2, 2)
    }

    fn jpeg() -> Vec<u8> {
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(
                &mut std::io::Cursor::new(&mut out),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        out
    }

    #[test]
    fn stores_png_and_jpeg() {
        let db = Db::open_in_memory().unwrap();
        let clock = FakeClock::at(Timestamp::from_millis(1));
        let images = ProfileImages::new(db.conn(), &clock);
        assert_eq!(images.get(ImageKind::Logo).unwrap(), None);
        images.set(ImageKind::Logo, png()).unwrap();
        images.set(ImageKind::Stamp, jpeg()).unwrap();
        assert_eq!(
            images.get(ImageKind::Logo).unwrap(),
            Some(Image {
                mime: "image/png",
                bytes: png()
            })
        );
        assert_eq!(
            images.get(ImageKind::Stamp).unwrap().unwrap().mime,
            "image/jpeg"
        );
        // Replacing and deleting.
        images.set(ImageKind::Logo, jpeg()).unwrap();
        assert_eq!(
            images.get(ImageKind::Logo).unwrap().unwrap().mime,
            "image/jpeg"
        );
        images.delete(ImageKind::Logo).unwrap();
        assert_eq!(images.get(ImageKind::Logo).unwrap(), None);
    }

    #[test]
    fn rejects_other_broken_and_large_files() {
        let db = Db::open_in_memory().unwrap();
        let clock = FakeClock::at(Timestamp::from_millis(1));
        let images = ProfileImages::new(db.conn(), &clock);
        let unsupported = CoreError::new("image.unsupported");
        assert_eq!(
            images
                .set(ImageKind::Signature, b"%PDF-1.7".to_vec())
                .unwrap_err(),
            unsupported
        );
        // The right signature bytes but not an image.
        assert_eq!(
            images
                .set(ImageKind::Signature, b"\x89PNG\r\n\x1a\nbroken".to_vec())
                .unwrap_err(),
            unsupported
        );
        let mut big = png();
        big.resize(MAX_BYTES + 1, 0);
        assert_eq!(
            images.set(ImageKind::Signature, big).unwrap_err(),
            CoreError::new("file.too_large").with("max", MAX_BYTES)
        );
    }
}
