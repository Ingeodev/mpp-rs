//! `mpp-rs` — bindings Rust de Rockchip MPP (`librockchip_mpp`) para el
//! ecosistema Afora, espejo de `rga-rs`: `mpp-sys` (FFI cruda) + `mpp`
//! (wrapper seguro).
//!
//! Clientes previstos: `afora-live-svc-rs` (H.264 → RTSP/WHEP, cliente #2) y
//! el cliente de storage MP4 (H.265, cliente #3). La librería es genérica:
//! la configuración específica de cada canal vive en el cliente.
//!
//! Estado v0: ciclo de vida del contexto, configuración e importación de
//! dma-buf. El ensamblado de `MppFrame` y la extracción de packets
//! (`encode`) llegan con la fase F6a de live-svc (requiere comandos `MpiCmd`
//! verificados por probe C contra `rk_mpi_cmd.h`).

mod buffer;
mod config;
mod error;

pub mod ffi {
    //! Re-export de la FFI cruda para clientes avanzados (spikes/HIL).
    pub use mpp_sys::*;
}

pub use buffer::{BufferGroup, ImportedBuffer};
pub use config::{Coding, EncoderConfig, RcMode};
pub use error::{Error, MppCallError, Result};

use std::fmt;
use std::ptr::NonNull;

use crate::error::check;

/// Geometría del frame de entrada (NV12) que recibe [`MppEncoder::encode`].
#[derive(Debug, Clone, Copy)]
pub struct FrameGeometry {
    pub width: u32,
    pub height: u32,
    pub hor_stride: u32,
    pub ver_stride: u32,
    /// Bytes válidos del buffer (≥ stride completo del plano Y + croma).
    pub buffer_size: usize,
}

/// Encoder de vídeo sobre MPP (contexto ENC + config).
///
/// Ciclo de vida: `new` hace `mpp_create` + `mpp_init(ENC, coding)` y monta
/// la config base (`prep:*`); `Drop` libera cfg + contexto.
pub struct MppEncoder {
    ctx: NonNull<std::ffi::c_void>,
    api: NonNull<ffi::MppApi>,
    cfg: NonNull<std::ffi::c_void>,
    config: EncoderConfig,
}

// SAFETY: MPP crea el encoder para usarse desde un hilo (patrón del SDK);
// el contexto no se comparte entre hilos desde este wrapper.
unsafe impl Send for MppEncoder {}

impl MppEncoder {
    /// Crea el encoder y aplica la configuración base.
    pub fn new(config: EncoderConfig) -> Result<Self> {
        config.validate()?;

        let mut ctx: ffi::MppCtx = std::ptr::null_mut();
        let mut api: *mut ffi::MppApi = std::ptr::null_mut();
        // SAFETY: out-params válidos.
        unsafe {
            check(ffi::mpp_create(&mut ctx, &mut api), "mpp_create")?;
            check(
                ffi::mpp_init(ctx, ffi::MPP_CTX_ENC, config.coding.as_raw()),
                "mpp_init",
            )?;
        }
        // SAFETY: mpp_create rellena ambos; api no nulo en éxito.
        let ctx = unsafe { NonNull::new_unchecked(ctx) };
        let api = unsafe { NonNull::new_unchecked(api) };

        let mut cfg: ffi::MppEncCfg = std::ptr::null_mut();
        // SAFETY: out-param válido.
        unsafe {
            check(ffi::mpp_enc_cfg_init(&mut cfg), "mpp_enc_cfg_init")?;
            let (w, h) = (config.width as i32, config.height as i32);
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"prep:width".as_ptr(), w),
                "set prep:width",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"prep:height".as_ptr(), h),
                "set prep:height",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"prep:hor_stride".as_ptr(), config.hor_stride as i32),
                "set prep:hor_stride",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"prep:ver_stride".as_ptr(), config.ver_stride as i32),
                "set prep:ver_stride",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"prep:format".as_ptr(), ffi::MPP_FMT_YUV420SP),
                "set prep:format",
            )?;
            // Rate control (claves verificadas en la fuente del SDK).
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:mode".as_ptr(), config.rc_mode.as_raw()),
                "set rc:mode",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:fps_in_num".as_ptr(), config.fps_in as i32),
                "set rc:fps_in_num",
            )?;
            // Denom=1 obligatorio: el default es 0 y el modelo RC hace
            // fps_out_num/fps_out_denom (hallazgo F6a: stat_rate init fail).
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:fps_in_denom".as_ptr(), 1),
                "set rc:fps_in_denom",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:fps_out_num".as_ptr(), config.fps_out as i32),
                "set rc:fps_out_num",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:fps_out_denom".as_ptr(), 1),
                "set rc:fps_out_denom",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:gop".as_ptr(), config.gop as i32),
                "set rc:gop",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(
                    cfg,
                    c"rc:bps_target".as_ptr(),
                    ((config.bps.0 + config.bps.1) / 2) as i32,
                ),
                "set rc:bps_target",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:bps_min".as_ptr(), config.bps.0 as i32),
                "set rc:bps_min",
            )?;
            check(
                ffi::mpp_enc_cfg_set_s32(cfg, c"rc:bps_max".as_ptr(), config.bps.1 as i32),
                "set rc:bps_max",
            )?;
            // Aplica la config al encoder (control por tabla MppApi).
            check(
                ((*api.as_ptr()).control)(ctx.as_ptr(), ffi::MPP_ENC_SET_CFG, cfg as ffi::MppParam),
                "MPP_ENC_SET_CFG",
            )?;
        }

        let encoder = Self {
            ctx,
            api,
            cfg: unsafe { NonNull::new_unchecked(cfg) },
            config,
        };
        Ok(encoder)
    }

    /// Encabezados del stream (VPS/SPS/PPS Annex-B) vía
    /// `MPP_ENC_GET_HDR_SYNC`. Llamar una vez tras `new`.
    ///
    /// Contrato del SDK (mpp_enc_impl.c): el encoder escribe el header en un
    /// **packet del usuario**; hay que inicializar el packet con buffer +
    /// length 0 antes del control. El buffer es DMA_HEAP (el allocator
    /// NORMAL/std está deshabilitado fuera de simulación en este SDK;
    /// hallazgo F6a).
    pub fn header(&mut self) -> Result<Vec<u8>> {
        let mut group: ffi::MppBufferGroup = std::ptr::null_mut();
        let mut buffer: ffi::MppBuffer = std::ptr::null_mut();
        let mut packet: ffi::MppPacket = std::ptr::null_mut();
        // SAFETY: out-params válidos; grupo/buffer/packet se liberan abajo.
        unsafe {
            check(
                ffi::mpp_buffer_group_get(
                    &mut group,
                    ffi::MPP_BUFFER_TYPE_DMA_HEAP,
                    ffi::MPP_BUFFER_INTERNAL,
                    c"afora-mpp-rs".as_ptr(),
                    c"header".as_ptr(),
                ),
                "mpp_buffer_group_get",
            )?;
            check(
                ffi::mpp_buffer_get_with_tag(
                    group,
                    &mut buffer,
                    8 * 1024,
                    c"afora-mpp-rs".as_ptr(),
                    c"header".as_ptr(),
                ),
                "mpp_buffer_get_with_tag",
            )?;
            check(
                ffi::mpp_packet_init_with_buffer(&mut packet, buffer),
                "mpp_packet_init_with_buffer",
            )?;
            ffi::mpp_packet_set_length(packet, 0);
            check(
                ((*self.api.as_ptr()).control)(
                    self.ctx.as_ptr(),
                    ffi::MPP_ENC_GET_HDR_SYNC,
                    packet as ffi::MppParam,
                ),
                "MPP_ENC_GET_HDR_SYNC",
            )?;
        }
        let out = Self::packet_bytes(packet);
        // SAFETY: packet/buffer creados por este método.
        unsafe {
            ffi::mpp_packet_deinit(&mut packet);
            ffi::mpp_buffer_put_with_caller(buffer, c"header".as_ptr());
            ffi::mpp_buffer_group_put(group);
        }
        out
    }

    /// Codifica el frame NV12 descrito por `geometry`, cuyo dma-buf ya está
    /// importado como `buffer` (zero-copy, D7). Devuelve el packet Annex-B.
    ///
    /// Vía real del SDK (mpi.c): `encode` simple no está implementado en
    /// este master (TODO) — se usa `encode_put_frame` + `encode_get_packet`
    /// (modo bloqueante por defecto).
    pub fn encode(&mut self, buffer: &ImportedBuffer, geometry: FrameGeometry) -> Result<Vec<u8>> {
        let mut frame: ffi::MppFrame = std::ptr::null_mut();
        let mut packet: ffi::MppPacket = std::ptr::null_mut();
        // SAFETY: out-params válidos; `buffer`/`geometry` describen memoria
        // dma-buf válida registrada en MPP.
        unsafe {
            check(ffi::mpp_frame_init(&mut frame), "mpp_frame_init")?;
            ffi::mpp_frame_set_width(frame, geometry.width);
            ffi::mpp_frame_set_height(frame, geometry.height);
            ffi::mpp_frame_set_hor_stride(frame, geometry.hor_stride);
            ffi::mpp_frame_set_ver_stride(frame, geometry.ver_stride);
            ffi::mpp_frame_set_fmt(frame, ffi::MPP_FMT_YUV420SP);
            ffi::mpp_frame_set_buf_size(frame, geometry.buffer_size);
            ffi::mpp_frame_set_color_range(frame, ffi::MPP_FRAME_RANGE_MPEG);
            ffi::mpp_frame_set_buffer(frame, buffer.raw());

            check(
                ((*self.api.as_ptr()).encode_put_frame)(self.ctx.as_ptr(), frame),
                "encode_put_frame",
            )?;
            // El frame (handle) se libera aquí; el buffer sigue siendo
            // propiedad del llamador (patrón del test del SDK).
            ffi::mpp_frame_deinit(&mut frame);

            check(
                ((*self.api.as_ptr()).encode_get_packet)(self.ctx.as_ptr(), &mut packet),
                "encode_get_packet",
            )?;
        }
        let out = Self::packet_bytes(packet);
        // SAFETY: packet entregado por get_packet.
        unsafe { ffi::mpp_packet_deinit(&mut packet) };
        out
    }

    fn packet_bytes(packet: ffi::MppPacket) -> Result<Vec<u8>> {
        // SAFETY: packet válido entregado por la API (get_pos/get_length son
        // las lecturas válidas para packets de encoder; get_data falla el
        // check interno de este SDK).
        let (ptr, size) =
            unsafe { (ffi::mpp_packet_get_pos(packet), ffi::mpp_packet_get_length(packet)) };
        if ptr.is_null() || size == 0 {
            return Ok(Vec::new());
        }
        // SAFETY: MPP garantiza `size` bytes legibles desde `pos`.
        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };
        Ok(bytes.to_vec())
    }

    pub fn config(&self) -> &EncoderConfig {
        &self.config
    }

    /// Acceso a la tabla `MppApi` para usos avanzados (spikes/HIL).
    ///
    /// # Safety
    /// El llamador debe respetar el contrato del SDK MPP (no llamar tras
    /// `Drop`, exclusión mutua de sus llamadas).
    pub unsafe fn api(&self) -> &ffi::MppApi {
        &*self.api.as_ptr()
    }

    /// Contexto crudo MPP (usos avanzados).
    ///
    /// # Safety
    /// Igual que [`Self::api`]: válido solo mientras el encoder vive.
    pub unsafe fn ctx(&self) -> ffi::MppCtx {
        self.ctx.as_ptr()
    }

    /// Config cruda `MppEncCfg` (claves string adicionales).
    ///
    /// # Safety
    /// Igual que [`Self::api`]: válido solo mientras el encoder vive.
    pub unsafe fn cfg(&self) -> ffi::MppEncCfg {
        self.cfg.as_ptr()
    }
}

impl fmt::Debug for MppEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MppEncoder")
            .field("config", &self.config)
            .finish()
    }
}

impl Drop for MppEncoder {
    fn drop(&mut self) {
        // SAFETY: punteros válidos mientras el struct vive; orden: soltar cfg
        // (referencia al ctx) antes que destruir el ctx.
        unsafe {
            ffi::mpp_enc_cfg_deinit(self.cfg.as_ptr());
            // mpp_destroy libera también el handle MppApi entregado.
            ffi::mpp_destroy(self.ctx.as_ptr());
        }
    }
}
