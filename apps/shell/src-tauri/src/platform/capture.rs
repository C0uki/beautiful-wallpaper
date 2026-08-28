//! Taking a picture of the screen.
//!
//! The order matters more than the mechanism. Showing a selection overlay and
//! *then* capturing puts the overlay in the picture; capturing first and
//! letting the user draw on the frozen frame does not, and has the further
//! advantage that what they select is what they saw rather than whatever the
//! screen has moved on to.
//!
//! Which means the shell's own transient surfaces have to be out of the way
//! before the shutter, and being out of the way is not the same as having been
//! told to go: hiding a window returns immediately, and the compositor has not
//! necessarily drawn the frame without it yet. `DwmFlush` waits for exactly
//! that, which is why there is no sleep here.

use std::path::Path;

use bw_core::capture::{dib_size, dib_stride, Rect};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    ROP_CODE, SRCCOPY,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

/// `CF_DIB`. Written out rather than pulling in the whole OLE namespace for
/// one number that has not changed since Windows 3.
const CF_DIB: u32 = 8;

/// Captured pixels, as RGBA, top row first.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Frame {
    /// The part of the frame inside `region`, as a frame of its own.
    ///
    /// The region is clamped first, so a selection that ran off the edge
    /// yields the part that existed rather than reading past the buffer.
    pub fn crop(&self, region: Rect) -> Option<Frame> {
        let bounds = Rect {
            x: 0,
            y: 0,
            width: self.width as i32,
            height: self.height as i32,
        };
        let region = region.clamp(bounds);
        if region.width <= 0 || region.height <= 0 {
            return None;
        }

        let (width, height) = (region.width as usize, region.height as usize);
        let source_stride = self.width as usize * 4;
        let mut pixels = Vec::with_capacity(width * height * 4);

        for row in 0..height {
            let start = (region.y as usize + row) * source_stride + region.x as usize * 4;
            pixels.extend_from_slice(&self.pixels[start..start + width * 4]);
        }

        Some(Frame {
            width: region.width as u32,
            height: region.height as u32,
            pixels,
        })
    }

    /// Writes the frame as a PNG.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }

        let image: image::RgbaImage =
            image::ImageBuffer::from_raw(self.width, self.height, self.pixels.clone())
                .ok_or_else(|| "the captured pixels do not match their size".to_owned())?;
        image
            .save(path)
            .map_err(|error| format!("could not write {}: {error}", path.display()))
    }

    /// Puts the frame on the clipboard as a device-independent bitmap.
    ///
    /// Twenty-four bits per pixel, not thirty-two: a screenshot has no
    /// meaningful alpha, and applications disagree about what to do with the
    /// fourth channel — some paste the image, some paste a black rectangle.
    pub fn to_clipboard(&self) -> Result<(), String> {
        let stride = dib_stride(self.width, 24);
        let header = std::mem::size_of::<BITMAPINFOHEADER>();
        let total = header + dib_size(self.width, self.height, 24);

        unsafe {
            let memory = GlobalAlloc(GMEM_MOVEABLE, total)
                .map_err(|error| format!("could not allocate for the clipboard: {error}"))?;

            let block = GlobalLock(memory).cast::<u8>();
            if block.is_null() {
                return Err("could not lock the clipboard's memory".to_owned());
            }

            let info = block.cast::<BITMAPINFOHEADER>();
            info.write(BITMAPINFOHEADER {
                biSize: header as u32,
                biWidth: self.width as i32,
                // Positive, so the rows go bottom-up: that is what a plain
                // `CF_DIB` means, and a negative height here is accepted by
                // some applications and rejected by others.
                biHeight: self.height as i32,
                biPlanes: 1,
                biBitCount: 24,
                biCompression: BI_RGB.0,
                ..Default::default()
            });

            let rows = std::slice::from_raw_parts_mut(block.add(header), total - header);
            rows.fill(0);

            for row in 0..self.height as usize {
                // Bottom-up: the last row of the image is the first in memory.
                let target = (self.height as usize - 1 - row) * stride;
                let source = row * self.width as usize * 4;

                for column in 0..self.width as usize {
                    let pixel = source + column * 4;
                    let out = target + column * 3;
                    // RGBA in, BGR out.
                    rows[out] = self.pixels[pixel + 2];
                    rows[out + 1] = self.pixels[pixel + 1];
                    rows[out + 2] = self.pixels[pixel];
                }
            }

            let _ = GlobalUnlock(memory);

            OpenClipboard(None)
                .map_err(|error| format!("could not open the clipboard: {error}"))?;
            let result = EmptyClipboard()
                .map_err(|error| format!("could not clear the clipboard: {error}"))
                .and_then(|()| {
                    SetClipboardData(CF_DIB, HANDLE(memory.0))
                        .map(|_| ())
                        .map_err(|error| format!("could not set the clipboard: {error}"))
                });
            let _ = CloseClipboard();

            // On success the clipboard owns the block; freeing it here would
            // hand every pasting application a dangling handle.
            result
        }
    }
}

/// `CF_UNICODETEXT`, for the same reason as [`CF_DIB`].
const CF_UNICODETEXT: u32 = 13;

/// Puts text on the clipboard.
///
/// Reading text off the screen and then not being able to paste it would
/// leave the user retyping what they just had recognised.
pub fn copy_text(text: &str) -> Result<(), String> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = std::mem::size_of_val(wide.as_slice());

    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes)
            .map_err(|error| format!("could not allocate for the clipboard: {error}"))?;

        let block = GlobalLock(memory).cast::<u16>();
        if block.is_null() {
            return Err("could not lock the clipboard's memory".to_owned());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), block, wide.len());
        let _ = GlobalUnlock(memory);

        OpenClipboard(None).map_err(|error| format!("could not open the clipboard: {error}"))?;
        let result = EmptyClipboard()
            .map_err(|error| format!("could not clear the clipboard: {error}"))
            .and_then(|()| {
                SetClipboardData(CF_UNICODETEXT, HANDLE(memory.0))
                    .map(|_| ())
                    .map_err(|error| format!("could not set the clipboard: {error}"))
            });
        let _ = CloseClipboard();
        result
    }
}

/// The primary monitor's rectangle, in physical pixels.
///
/// One monitor only, deliberately: a window has a single scale factor, so an
/// overlay spanning two monitors at different scales cannot map what was drawn
/// on it back to pixels on both.
pub fn primary_bounds() -> Option<Rect> {
    let monitors = crate::platform::win::monitors();
    let monitor = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())?;

    Some(Rect {
        x: monitor.x,
        y: monitor.y,
        width: monitor.width,
        height: monitor.height,
    })
}

/// Waits for the compositor to finish the frame currently being drawn.
///
/// Called after hiding the shell's own overlays and before the shutter. Twice,
/// because the first returns as soon as the frame in flight is done — which
/// may still be the one that had the overlay in it.
pub fn settle() {
    unsafe {
        let _ = DwmFlush();
        let _ = DwmFlush();
    }
}

/// Copies a rectangle of the screen.
pub fn grab(region: Rect) -> Result<Frame, String> {
    if region.width <= 0 || region.height <= 0 {
        return Err("there is nothing to capture".to_owned());
    }
    let (width, height) = (region.width, region.height);

    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("could not read the screen".to_owned());
        }

        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, width, height);

        let captured = (|| {
            if memory.is_invalid() || bitmap.is_invalid() {
                return Err("could not make room for the capture".to_owned());
            }

            let previous = SelectObject(memory, bitmap);
            // `CAPTUREBLT` is what includes layered windows — without it,
            // anything drawn with transparency comes back as a hole.
            let copied = BitBlt(
                memory,
                0,
                0,
                width,
                height,
                screen,
                region.x,
                region.y,
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
            );
            // Deselected before reading: `GetDIBits` will not touch a bitmap
            // that is still selected into the device context it is given.
            SelectObject(memory, previous);
            copied.map_err(|error| format!("could not copy the screen: {error}"))?;

            read_pixels(memory, bitmap, width as u32, height as u32)
        })();

        if !bitmap.is_invalid() {
            let _ = DeleteObject(bitmap);
        }
        if !memory.is_invalid() {
            let _ = DeleteDC(memory);
        }
        ReleaseDC(None, screen);

        captured
    }
}

/// Reads a bitmap's pixels out as RGBA, top row first.
unsafe fn read_pixels(
    device: windows::Win32::Graphics::Gdi::HDC,
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    width: u32,
    height: u32,
) -> Result<Frame, String> {
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // Negative asks for top-down rows; the default is bottom-up, which
            // would deliver the screen upside down.
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let copied = GetDIBits(
        device,
        bitmap,
        0,
        height,
        Some(buffer.as_mut_ptr().cast()),
        &mut header,
        DIB_RGB_COLORS,
    );
    if copied == 0 {
        return Err("could not read the captured pixels".to_owned());
    }

    // GDI hands back BGRA; everything downstream wants RGBA. The alpha channel
    // is whatever happened to be in the bitmap, so it is set rather than kept:
    // a screenshot is opaque, and a zero alpha would save a blank PNG.
    for pixel in buffer.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 0xff;
    }

    Ok(Frame {
        width,
        height,
        pixels: buffer,
    })
}
