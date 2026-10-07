//! What the speakers are playing, for the visualiser.
//!
//! WASAPI's loopback mode records the default output device's mix — whatever
//! every application together is sending to it — without a virtual cable or
//! any other driver. The capture runs on a thread of its own and hands over
//! bars about thirty times a second; it only runs while something on screen
//! is drawing them.
//!
//! Loopback delivers nothing at all while nothing plays, rather than
//! silence, so a quiet stream is read as zeros and sent once, and then the
//! thread stays quiet too.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows::core::{Result, GUID};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
    WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
};

/// Bars per frame.
pub const BARS: usize = 32;

/// `WAVE_FORMAT_IEEE_FLOAT`, and the same as a `WAVEFORMATEXTENSIBLE` subtype.
const FLOAT_TAG: u16 = 3;
const EXTENSIBLE_TAG: u16 = 0xFFFE;
const FLOAT_SUBTYPE: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

/// A running capture. Dropping it stops the thread.
pub struct Loopback {
    stop: Arc<AtomicBool>,
}

impl Drop for Loopback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Starts capturing, calling `on_bars` with each frame of bars.
///
/// Follows the default output device: a capture that finds the default has
/// moved — headphones plugged in, a speaker picked in the volume flyout —
/// ends, and the next one listens to the new device.
pub fn start(on_bars: impl Fn(Vec<f32>) + Send + 'static) -> Loopback {
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let _ = std::thread::Builder::new()
        .name("bw-loopback".to_owned())
        .spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            while !stopping.load(Ordering::Relaxed) {
                let device = default_output();
                if let Err(error) = capture(&stopping, &on_bars) {
                    tracing::warn!(%error, "the visualiser could not listen to the output");
                    // Tried again on another device only: the same one would
                    // fail the same way. A device that went away mid-capture
                    // is one, and the new default is taken up here.
                    while !stopping.load(Ordering::Relaxed) && default_output() == device {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        });
    Loopback { stop }
}

/// The id of the default output device, if there is one.
unsafe fn default_output() -> Option<String> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let id = enumerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .ok()?
        .GetId()
        .ok()?;
    let text = id.to_string().ok();
    CoTaskMemFree(Some(id.0 as _));
    text
}

/// Listens to the default output until told to stop, or until the default is
/// another device.
unsafe fn capture(stop: &AtomicBool, on_bars: &dyn Fn(Vec<f32>)) -> Result<()> {
    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
    let listening = default_output();
    let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

    let format = client.GetMixFormat()?;
    let header: WAVEFORMATEX = std::ptr::read_unaligned(format);
    let float = header.wFormatTag == FLOAT_TAG
        || (header.wFormatTag == EXTENSIBLE_TAG && {
            let extensible: WAVEFORMATEXTENSIBLE = std::ptr::read_unaligned(format.cast());
            // Copied out first: the struct is packed, and comparing borrows.
            let subtype = extensible.SubFormat;
            subtype == FLOAT_SUBTYPE
        });
    let channels = usize::from(header.nChannels.max(1));
    let rate = header.nSamplesPerSec;
    // The shared-mode mix is 32-bit float on every Windows that has shipped;
    // anything else is left undrawn rather than misread.
    let initialised = if float && header.wBitsPerSample == 32 {
        // 100 ms of buffer, in 100 ns units: room for a late wake-up.
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            1_000_000,
            0,
            format,
            None,
        )
    } else {
        Err(windows::Win32::Foundation::E_NOTIMPL.into())
    };
    CoTaskMemFree(Some(format.cast()));
    initialised?;

    let capture: IAudioCaptureClient = client.GetService()?;
    client.Start()?;

    let mut window: VecDeque<f32> = VecDeque::with_capacity(bw_core::spectrum::WINDOW);
    let mut last_sent = Instant::now();
    let mut last_heard = Instant::now();
    let mut quiet = false;
    let mut last_looked = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));

        if last_looked.elapsed() > Duration::from_secs(1) {
            last_looked = Instant::now();
            if default_output() != listening {
                break;
            }
        }

        while capture.GetNextPacketSize()? > 0 {
            let mut data = std::ptr::null_mut();
            let mut frames = 0u32;
            let mut flags = 0u32;
            capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
            let samples =
                std::slice::from_raw_parts(data.cast::<f32>(), frames as usize * channels);
            let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
            for frame in samples.chunks_exact(channels) {
                let mono = if silent {
                    0.0
                } else {
                    frame.iter().sum::<f32>() / channels as f32
                };
                if window.len() == bw_core::spectrum::WINDOW {
                    window.pop_front();
                }
                window.push_back(mono);
            }
            capture.ReleaseBuffer(frames)?;
            last_heard = Instant::now();
        }

        if last_sent.elapsed() < Duration::from_millis(33) {
            continue;
        }
        // A packet can be a little late while something is playing; a
        // quarter of a second without one is nothing playing.
        if last_heard.elapsed() > Duration::from_millis(250) {
            // Nothing playing: one frame of flat bars, then nothing.
            if !quiet {
                quiet = true;
                window.clear();
                on_bars(vec![0.0; BARS]);
            }
            continue;
        }
        quiet = false;
        last_sent = Instant::now();
        let samples: Vec<f32> = window.iter().copied().collect();
        on_bars(bw_core::spectrum::bars(&samples, rate, BARS));
    }

    let _ = client.Stop();
    Ok(())
}
