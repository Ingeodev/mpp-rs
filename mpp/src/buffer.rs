//! Importación de buffers dma-buf externos (fd) como `MppBuffer`.
//!
//! Caso principal del ecosistema: los frames NV12 llegan como fd de dma-buf
//! (del camera-adapter vía RGA o DRM — ver D7 de live-svc) y MPP los consume
//! in-place, sin copia.

use std::fmt;
use std::sync::Arc;

use mpp_sys as ffi;

use crate::error::{check, Result};

/// Un buffer externo (dma-buf por fd) registrado en MPP.
///
/// Al soltarlo se devuelve al grupo (`mpp_buffer_put`); el fd en sí es
/// propiedad del llamador (el pozó de fds del camera-adapter).
pub struct ImportedBuffer {
    raw: ffi::MppBuffer,
    // El grupo debe sobrevivir al buffer.
    _group: Arc<BufferGroup>,
}

impl ImportedBuffer {
    /// Handle crudo de MPP (para `mpp_frame_set_buffer`).
    pub fn raw(&self) -> ffi::MppBuffer {
        self.raw
    }

    /// fd del dma-buf asociado (recuperado de MPP, -1 si no aplica).
    pub fn fd(&self) -> i32 {
        let mut info = ffi::MppBufferInfo::dma_fd(-1, 0);
        // El tag/caller son solo para trazas internas de MPP.
        // SAFETY: `raw` es un MppBuffer válido entregado por `import_with_tag`.
        let ret = unsafe {
            ffi::mpp_buffer_info_get_with_caller(
                self.raw,
                &mut info,
                c"mpp-rs".as_ptr(),
            )
        };
        if ret == ffi::MPP_OK {
            info.fd
        } else {
            -1
        }
    }
}

impl fmt::Debug for ImportedBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportedBuffer").field("fd", &self.fd()).finish()
    }
}

impl Drop for ImportedBuffer {
    fn drop(&mut self) {
        // SAFETY: puntero obtenido de `mpp_buffer_import_with_tag` y no
        // soltado antes (unicidad garantizada por este wrapper).
        unsafe { ffi::mpp_buffer_put_with_caller(self.raw, c"mpp-rs".as_ptr()) };
    }
}

/// Grupo de buffers externo de MPP.
///
/// Threading: MPP no exige exclusión externa para estas llamadas, pero el
/// ciclo de vida compartido entre encoder y buffers se modela con `Arc`.
pub struct BufferGroup {
    raw: ffi::MppBufferGroup,
}

// SAFETY: los handle de MPP se pasan por valor y su uso está acotado a las
// llamadas FFI declaradas; no se comparte estado Rust mutable.
unsafe impl Send for BufferGroup {}
unsafe impl Sync for BufferGroup {}

impl BufferGroup {
    /// Grupo para buffers externos importados (dma-buf del llamador).
    pub fn external_dma() -> Result<Arc<Self>> {
        // MPP_BUFFER_TYPE_EXT_DMA: "the DMABUF come from the application"
        // (mpp_buffer.h). El import exige modo EXTERNAL (mpp_buffer.c
        // rechaza group mode INTERNAL en imports — hallazgo F6a).
        Self::with(ffi::MPP_BUFFER_TYPE_EXT_DMA, ffi::MPP_BUFFER_EXTERNAL)
    }

    fn with(buffer_type: i32, mode: i32) -> Result<Arc<Self>> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: out-params válidos; tag/caller son literales estáticos.
        unsafe {
            check(
                ffi::mpp_buffer_group_get(
                    &mut raw,
                    buffer_type,
                    mode,
                    c"afora-mpp-rs".as_ptr(),
                    c"BufferGroup::with".as_ptr(),
                ),
                "mpp_buffer_group_get",
            )?;
        }
        Ok(Arc::new(Self { raw }))
    }

    /// Importa un dma-buf por fd como `MppBuffer` de MPP.
    pub fn import_fd(self: &Arc<Self>, fd: i32, size: usize) -> Result<ImportedBuffer> {
        let mut info = ffi::MppBufferInfo::dma_fd(fd, size);
        let mut buffer = std::ptr::null_mut();
        // SAFETY: out-params válidos; el info es un struct local.
        unsafe {
            check(
                ffi::mpp_buffer_import_with_tag(
                    self.raw,
                    &mut info,
                    &mut buffer,
                    c"afora-mpp-rs".as_ptr(),
                    c"import_fd".as_ptr(),
                ),
                "mpp_buffer_import_with_tag",
            )?;
        }
        Ok(ImportedBuffer {
            raw: buffer,
            _group: Arc::clone(self),
        })
    }
}

impl Drop for BufferGroup {
    fn drop(&mut self) {
        // SAFETY: último propietario del grupo (Arc count == 0).
        unsafe { ffi::mpp_buffer_group_put(self.raw) };
    }
}
