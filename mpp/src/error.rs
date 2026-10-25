//! Errores del wrapper seguro sobre MPP.

use std::fmt;

use mpp_sys::MPP_RET;

/// Error de una llamada a la API de MPP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MppCallError {
    /// Código `MPP_RET` devuelto por la librería (0 = OK, negativo = error).
    pub code: i32,
    /// Llamada que falló (p. ej. `"mpp_init"`).
    pub call: &'static str,
}

impl fmt::Display for MppCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} falló con código {}", self.call, self.code)
    }
}

impl std::error::Error for MppCallError {}

#[derive(Debug)]
pub enum Error {
    /// Llamada MPP con código de error.
    Mpp(MppCallError),
    /// Parámetro inválido para MPP (dimensiones, formato, etc.).
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Mpp(e) => e.fmt(f),
            Error::Invalid(why) => write!(f, "argumento inválido: {why}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Convierte un `MPP_RET` en `Result`, etiquetando la llamada fallida.
pub(crate) fn check(ret: MPP_RET, call: &'static str) -> Result<()> {
    if ret == mpp_sys::MPP_OK {
        Ok(())
    } else {
        Err(Error::Mpp(MppCallError { code: ret, call }))
    }
}
