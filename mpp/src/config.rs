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

/// Configuración genérica de encoder (dimensiones + rate control).
///
/// Las claves de configuración de MPP son strings (`"prep:width"`, …);
/// aquí se modela el subconjunto que usa el ecosistema Afora.
#[derive(Debug, Clone, Copy)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub coding: Coding,
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
        if self.gop == 0 || self.fps_out == 0 || self.fps_in == 0 {
            return Err(crate::Error::Invalid("fps/gop nulos"));
        }
        if self.bps.0 > self.bps.1 {
            return Err(crate::Error::Invalid("bps.min > bps.max"));
        }
        Ok(())
    }
}
