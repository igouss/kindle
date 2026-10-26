#![allow(unsafe_code)]

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::FileExt;

use nix::libc::c_ulong;
use nix::{ioctl_read_bad, ioctl_readwrite, ioctl_write_ptr};

use crate::canvas::{Canvas, Geometry};

const DEVICE: &str = "/dev/fb0";
const GRAYSCALE_8BIT: u32 = 1;
const WAVEFORM_MODE_DU: u32 = 1;
const WAVEFORM_MODE_GC16: u32 = 2;
const WAVEFORM_MODE_GC16_FAST: u32 = 3;
const UPDATE_MODE_PARTIAL: u32 = 0;
const UPDATE_MODE_FULL: u32 = 1;
const TEMP_USE_AUTO: i32 = 0x1001;

#[repr(C)]
#[derive(Default)]
struct FbBitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
#[derive(Default)]
struct FbVarScreeninfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Default)]
struct FbFixScreeninfo {
    id: [u8; 16],
    smem_start: c_ulong,
    smem_len: u32,
    kind: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

#[repr(C)]
#[derive(Default)]
struct MxcfbRect {
    top: u32,
    left: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Default)]
struct MxcfbAltBufferData {
    phys_addr: u32,
    width: u32,
    height: u32,
    alt_update_region: MxcfbRect,
}

#[repr(C)]
#[derive(Default)]
struct MxcfbUpdateData {
    update_region: MxcfbRect,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
    hist_bw_waveform_mode: u32,
    hist_gray_waveform_mode: u32,
    temp: i32,
    flags: u32,
    alt_buffer_data: MxcfbAltBufferData,
}

#[repr(C)]
#[derive(Default)]
struct MxcfbUpdateMarkerData {
    update_marker: u32,
    collision_test: u32,
}

const _: () = assert!(size_of::<FbVarScreeninfo>() == 160);
const _: () =
    assert!(size_of::<FbFixScreeninfo>() == if size_of::<c_ulong>() == 4 { 68 } else { 80 });
const _: () = assert!(size_of::<MxcfbUpdateData>() == 72);
const _: () = assert!(size_of::<MxcfbUpdateMarkerData>() == 8);

ioctl_read_bad!(fbioget_vscreeninfo, 0x4600, FbVarScreeninfo);
ioctl_read_bad!(fbioget_fscreeninfo, 0x4602, FbFixScreeninfo);
ioctl_write_ptr!(mxcfb_send_update, b'F', 0x2E, MxcfbUpdateData);
ioctl_readwrite!(
    mxcfb_wait_for_update_complete,
    b'F',
    0x2F,
    MxcfbUpdateMarkerData
);

pub enum Fault {
    Open(io::Error),
    Query(nix::Error),
    Unsupported { bits_per_pixel: u32, grayscale: u32 },
    Mismatch { expected: Geometry, found: Geometry },
    Write(io::Error),
    Refresh(nix::Error),
    Wait(nix::Error),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Open(e) => write!(f, "open {DEVICE}: {e}"),
            Fault::Query(e) => write!(f, "query {DEVICE} screen info: {e}"),
            Fault::Unsupported {
                bits_per_pixel,
                grayscale,
            } => {
                write!(
                    f,
                    "{DEVICE} is {bits_per_pixel} bpp, grayscale {grayscale}; only 8 bpp non-inverted gray is drawn"
                )
            }
            Fault::Mismatch { expected, found } => {
                write!(f, "canvas {found:?} does not match {DEVICE} {expected:?}")
            }
            Fault::Write(e) => write!(f, "write {DEVICE}: {e}"),
            Fault::Refresh(e) => write!(f, "MXCFB_SEND_UPDATE: {e}"),
            Fault::Wait(e) => write!(f, "MXCFB_WAIT_FOR_UPDATE_COMPLETE: {e}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    Flash,
    Quiet,
}

pub struct Epdc {
    device: File,
    geometry: Geometry,
    origin: u64,
}

impl Epdc {
    pub fn open() -> Result<Epdc, Fault> {
        let device: File = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEVICE)
            .map_err(Fault::Open)?;
        let fd: RawFd = device.as_raw_fd();
        let mut var: FbVarScreeninfo = FbVarScreeninfo::default();
        let mut fix: FbFixScreeninfo = FbFixScreeninfo::default();
        unsafe { fbioget_vscreeninfo(fd, &mut var) }.map_err(Fault::Query)?;
        unsafe { fbioget_fscreeninfo(fd, &mut fix) }.map_err(Fault::Query)?;
        if var.bits_per_pixel != 8 || var.grayscale != GRAYSCALE_8BIT {
            return Err(Fault::Unsupported {
                bits_per_pixel: var.bits_per_pixel,
                grayscale: var.grayscale,
            });
        }
        let geometry: Geometry = Geometry {
            width: var.xres,
            height: var.yres,
            stride: fix.line_length,
        };
        let origin: u64 =
            u64::from(var.yoffset) * u64::from(fix.line_length) + u64::from(var.xoffset);
        Ok(Epdc {
            device,
            geometry,
            origin,
        })
    }

    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    pub fn present(&self, canvas: &Canvas, marker: u32, refresh: Refresh) -> Result<i32, Fault> {
        if canvas.geometry() != self.geometry {
            return Err(Fault::Mismatch {
                expected: self.geometry,
                found: canvas.geometry(),
            });
        }
        self.device
            .write_all_at(canvas.pixels(), self.origin)
            .map_err(Fault::Write)?;
        let fd: RawFd = self.device.as_raw_fd();
        let update: MxcfbUpdateData = MxcfbUpdateData {
            update_region: MxcfbRect {
                top: 0,
                left: 0,
                width: self.geometry.width,
                height: self.geometry.height,
            },
            waveform_mode: WAVEFORM_MODE_GC16,
            update_mode: match refresh {
                Refresh::Flash => UPDATE_MODE_FULL,
                Refresh::Quiet => UPDATE_MODE_PARTIAL,
            },
            update_marker: marker,
            hist_bw_waveform_mode: WAVEFORM_MODE_DU,
            hist_gray_waveform_mode: WAVEFORM_MODE_GC16_FAST,
            temp: TEMP_USE_AUTO,
            ..MxcfbUpdateData::default()
        };
        unsafe { mxcfb_send_update(fd, &update) }.map_err(Fault::Refresh)?;
        let mut done: MxcfbUpdateMarkerData = MxcfbUpdateMarkerData {
            update_marker: marker,
            collision_test: 0,
        };
        unsafe { mxcfb_wait_for_update_complete(fd, &mut done) }.map_err(Fault::Wait)
    }
}
