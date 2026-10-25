//! Configuración del encoder, independiente de cualquier cliente
//! (live-svc decide sus valores concretos y su port `Encoder`).

use mpp_sys::{MPP_VIDEO_CodingAVC, MPP_VIDEO_CodingHEVC};

/// Codec de vídeo soportado por el wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coding {
    /// H.264/AVC (cliente #2: canal de panel RTSP/WHEP).
    Avc,
    /// H.265/HEVC (cliente #3: storage MP4).
    Hevc,
}

impl Coding {
    pub const fn as_raw(self) -> i32 {
        match self {
            Coding::Avc => MPP_VIDEO_CodingAVC,
            Coding::Hevc => MPP_VIDEO_CodingHEVC,
        }
    }
}

/// Modo de rate-control (`rc:mode` del SDK; enum `RcMode` de mpp_rc_api.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcMode {
    /// Variable bitrate (0).
    Vbr,
    /// Constant bitrate (1) — estable para preview en vivo.
    Cbr,
    /// QP fijo (2) — sin control de bitrate.
    FixQp,
}

impl RcMode {
    pub const fn as_raw(self) -> i32 {
        match self {
            RcMode::Vbr => mpp_sys::RC_VBR,
            RcMode::Cbr => mpp_sys::RC_CBR,
            RcMode::FixQp => mpp_sys::RC_FIXQP,
        }
    }
}

/// Configuración genérica de encoder (dimensiones + rate control).
///
/// Las claves de configuración de MPP son strings (`"prep:width"`, …);
/// aquí se modela el subconjunto que usa el ecosistema Afora. Los nombres de
/// clave están verificados contra la fuente del SDK (`test/mpi_rc2_test.c`,
/// `mpp/codec/mpp_enc_impl.c`).
#[derive(Debug, Clone, Copy)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    /// Stride horizontal del buffer de entrada (px; NV12 = bytes Y por fila).
    pub hor_stride: u32,
    /// Stride vertical (rows; NV12 incluye el plano de croma).
    pub ver_stride: u32,
    pub coding: Coding,
    /// Modo de rate-control.
    pub rc_mode: RcMode,
    /// Frames por segundo de entrada (numerador implícito 1).
    pub fps_in: u32,
    /// Frames por segundo de salida.
    pub fps_out: u32,
    /// Intervalo de frames intra (GOP).
    pub gop: u32,
    /// Rango de bitrate objetivo en bps.
    pub bps: (u32, u32),
}

impl EncoderConfig {
    pub fn validate(&self) -> Result<(), crate::Error> {
        if self.width == 0 || self.height == 0 {
            return Err(crate::Error::Invalid("dimensiones nulas"));
        }
        if self.hor_stride == 0 || self.ver_stride == 0 {
            return Err(crate::Error::Invalid("strides nulos"));
        }
        if self.gop == 0 || self.fps_out == 0 || self.fps_in == 0 {
            return Err(crate::Error::Invalid("fps/gop nulos"));
        }
        if self.bps.0 > self.bps.1 {
            return Err(crate::Error::Invalid("bps.min > bps.max"));
        }
        Ok(())
    }
}
