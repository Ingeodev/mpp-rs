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
pub use config::{Coding, EncoderConfig};
pub use error::{Error, MppCallError, Result};

use std::fmt;
use std::ptr::NonNull;

use crate::error::check;

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
        }

        let encoder = Self {
            ctx,
            api,
            cfg: unsafe { NonNull::new_unchecked(cfg) },
            config,
        };

        // La config se aplica con mpp_control(MPP_ENC_SET_CFG) cuando estén
        // verificados los comandos MpiCmd (probe C, fase F6a).
        Ok(encoder)
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
