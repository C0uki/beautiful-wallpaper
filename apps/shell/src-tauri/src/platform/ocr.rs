//! Reading the text out of a captured region.
//!
//! Windows has a recogniser built in, which is the whole reason this is
//! possible without a dependency. What it does not have is every language: a
//! recogniser exists only for languages whose pack is installed, and asking
//! for one that is not returns nothing rather than failing loudly.
//!
//! So "there is no recogniser on this machine" is a first-class outcome here,
//! the way "this display has no brightness control" is elsewhere — the
//! feature is withheld rather than offered and then found dead.

use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

use crate::platform::capture::Frame;

/// Why there is no text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OcrError {
    /// No recogniser could be made — no language pack is installed, or the
    /// one named in the config is not among them.
    Unavailable,
    /// A recogniser ran and found nothing. Not a failure: an empty region.
    NothingFound,
    /// Something went wrong that the user can do nothing about.
    Failed(String),
}

/// Reads a frame, in `language` or in whatever the user reads.
pub fn read(frame: &Frame, language: &str) -> Result<String, OcrError> {
    let engine = engine(language).ok_or(OcrError::Unavailable)?;

    // The recogniser refuses anything above its own limit, so an oversized
    // selection is scaled down rather than rejected: a slightly softer image
    // reads better than no image at all.
    let limit = OcrEngine::MaxImageDimension().unwrap_or(4_000);
    let scaled = downscale(frame, limit);
    let source = scaled.as_ref().unwrap_or(frame);

    let bitmap = to_bitmap(source).map_err(|error| OcrError::Failed(error.to_string()))?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .and_then(|operation| operation.get())
        .map_err(|error| OcrError::Failed(error.to_string()))?;

    let tag = engine
        .RecognizerLanguage()
        .and_then(|found| found.LanguageTag())
        .map(|tag| tag.to_string())
        .unwrap_or_default();

    let mut lines: Vec<String> = Vec::new();
    if let Ok(found) = result.Lines() {
        for line in found {
            if let Ok(text) = line.Text() {
                lines.push(text.to_string());
            }
        }
    }

    let text = bw_core::ocr::join_lines(&lines, &tag);
    if bw_core::ocr::is_meaningful(&text) {
        Ok(text)
    } else {
        Err(OcrError::NothingFound)
    }
}

/// Whether this machine can read anything at all.
///
/// Asked before the feature is offered, so a machine with no language pack
/// does not get a menu entry that cannot work.
pub fn is_available(language: &str) -> bool {
    engine(language).is_some()
}

/// The recogniser to use, or `None` if there is not one.
fn engine(language: &str) -> Option<OcrEngine> {
    if !language.trim().is_empty() {
        let tag = Language::CreateLanguage(&HSTRING::from(language)).ok()?;
        // A configured language that is not installed is worth failing on
        // rather than quietly substituting: the user asked for that one.
        return OcrEngine::TryCreateFromLanguage(&tag).ok();
    }
    OcrEngine::TryCreateFromUserProfileLanguages().ok()
}

/// A copy no larger than `limit` on its longest side, or `None` if it already
/// fits.
fn downscale(frame: &Frame, limit: u32) -> Option<Frame> {
    let longest = frame.width.max(frame.height);
    if longest <= limit || limit == 0 {
        return None;
    }

    let ratio = f64::from(limit) / f64::from(longest);
    let width = ((f64::from(frame.width) * ratio) as u32).max(1);
    let height = ((f64::from(frame.height) * ratio) as u32).max(1);

    let image: image::RgbaImage =
        image::ImageBuffer::from_raw(frame.width, frame.height, frame.pixels.clone())?;
    let resized = image::imageops::resize(
        &image,
        width,
        height,
        image::imageops::FilterType::CatmullRom,
    );

    Some(Frame {
        width,
        height,
        pixels: resized.into_raw(),
    })
}

/// An RGBA frame as the BGRA bitmap the recogniser wants.
fn to_bitmap(frame: &Frame) -> windows::core::Result<SoftwareBitmap> {
    let mut bgra = frame.pixels.clone();
    for pixel in bgra.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }

    // There is no public constructor for an `IBuffer` over borrowed bytes, so
    // the standard route is to write them into one.
    let writer = DataWriter::new()?;
    writer.WriteBytes(&bgra)?;
    let buffer = writer.DetachBuffer()?;

    SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        frame.width as i32,
        frame.height as i32,
    )
}
