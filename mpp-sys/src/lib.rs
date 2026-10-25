//! Bindings FFI crudos (manuales) de Rockchip MPP (`librockchip_mpp`, SDK
//! open-source rockchip-linux/mpp).
//!
//! Solo se declaran los símbolos que usa `mpp-rs`; cada tipo/constante está
//! transcrita y verificada contra los headers del SDK instalado
//! (`~/.local/include/rockchip/`, versión 1.3.10 — ver README de la raíz,
//! sección P0-a).
//!
//! Convenciones:
//! - Tipos opacos (`MppCtx`, `MppPacket`, …) = `*mut c_void`, como en los headers.
//! - Enums C = `c_int` + constantes con el valor numérico verificado.
//! - Los comandos `MpiCmd` (`MPP_ENC_SET_HEADER_MODE`, `MPP_ENC_GET_HDR_SYNC`, …)
//!   NO se hardcodean a ciegas: sus valores dependen del orden del enum en
//!   `rk_mpi_cmd.h`; se añadirán generados por un probe C (fase F6a de live-svc).

// Nombres fieles a los headers C del SDK (MPP_VIDEO_CodingAVC, …).
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_void};

// rk_type.h / mpp_err.h — código de retorno.
pub type MPP_RET = c_int;
pub const MPP_OK: MPP_RET = 0; // RK_OK

// rk_mpi_cmd.h — MpiCmd. Valores generados por **probe C** contra el header
// del SDK instalado (2026-10-25, v1.3.10): gcc -I ~/.local/include/rockchip
// con un main() que imprime los enum. NO adivinar; si cambia el SDK,
// re-ejecutar el probe.
pub const MPP_ENC_SET_CFG: MpiCmd = 3276801; // set MppEncCfg structure
pub const MPP_ENC_GET_HDR_SYNC: MpiCmd = 3276813; // get vps/sps/pps (MppPacket)
pub const MPP_ENC_SET_HEADER_MODE: MpiCmd = 3309569; // set MppEncHeaderMode

// mpp_frame.h — valores de formato/rango (probe C, 2026-10-25).
pub const MPP_FMT_YUV420SP: c_int = 0; // NV12 (MPP_FRAME_FMT_YUV + 0)
pub const MPP_FRAME_RANGE_MPEG: c_int = 1; // limited range (cámara BT601 limited)

// inc/mpp_rc_api.h — RcMode (enum por posición; valores usados con la clave
// string "rc:mode" de mpp_enc_cfg).
pub const RC_VBR: c_int = 0;
pub const RC_CBR: c_int = 1;
pub const RC_FIXQP: c_int = 2;

// rk_type.h — tipos opacos.
pub type MppCtx = *mut c_void;
pub type MppPacket = *mut c_void;
pub type MppFrame = *mut c_void;
pub type MppParam = *mut c_void;
pub type MppTask = *mut c_void;
pub type MppBuffer = *mut c_void;
pub type MppBufferGroup = *mut c_void;
pub type MppEncCfg = *mut c_void;
pub type MpiCmd = c_int;

// rk_type.h — MppCtxType (enum, valores por posición).
pub const MPP_CTX_DEC: c_int = 0;
pub const MPP_CTX_ENC: c_int = 1;
pub const MPP_CTX_ISP: c_int = 2;

// rk_type.h — MppCodingType (enum "sync con omx_video.h"; valores por posición).
pub const MPP_VIDEO_CodingAVC: c_int = 7; // H.264/AVC
pub const MPP_VIDEO_CodingHEVC: c_int = 15; // H.265/HEVC

// mpp_buffer.h — MppBufferType.
pub const MPP_BUFFER_TYPE_NORMAL: c_int = 0;
pub const MPP_BUFFER_TYPE_ION: c_int = 1;
pub const MPP_BUFFER_TYPE_EXT_DMA: c_int = 2;
pub const MPP_BUFFER_TYPE_DRM: c_int = 3;
pub const MPP_BUFFER_TYPE_DMA_HEAP: c_int = 4;

// mpp_buffer.h — MppBufferMode.
pub const MPP_BUFFER_INTERNAL: c_int = 0;
pub const MPP_BUFFER_EXTERNAL: c_int = 1;

/// mpp_buffer.h lineas 174–181 — `MppBufferInfo` (layout verificado).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MppBufferInfo {
    /// MppBufferType
    pub type_: c_int,
    pub size: usize,
    pub ptr: *mut c_void,
    pub hnd: *mut c_void,
    pub fd: c_int,
    pub index: c_int,
}

impl MppBufferInfo {
    /// `MppBufferInfo` listo para importar un dma-buf por fd.
    pub const fn dma_fd(fd: c_int, size: usize) -> Self {
        Self {
            type_: MPP_BUFFER_TYPE_EXT_DMA,
            size,
            ptr: std::ptr::null_mut(),
            hnd: std::ptr::null_mut(),
            fd,
            index: 0,
        }
    }
}

// rk_mpi.h lineas 73–229 — `MppApi` (tabla de funciones; orden de campos
// verificado contra el header). Acceso: ctx devuelto por `mpp_create`.
#[repr(C)]
pub struct MppApi {
    pub size: u32,
    pub version: u32,

    // Flujo de datos simple.
    pub decode: unsafe extern "C" fn(MppCtx, MppPacket, *mut MppFrame) -> MPP_RET,
    pub decode_put_packet: unsafe extern "C" fn(MppCtx, MppPacket) -> MPP_RET,
    pub decode_get_frame: unsafe extern "C" fn(MppCtx, *mut MppFrame) -> MPP_RET,
    pub encode: unsafe extern "C" fn(MppCtx, MppFrame, *mut MppPacket) -> MPP_RET,
    pub encode_put_frame: unsafe extern "C" fn(MppCtx, MppFrame) -> MPP_RET,
    pub encode_get_packet: unsafe extern "C" fn(MppCtx, *mut MppPacket) -> MPP_RET,

    // ISP.
    pub isp: unsafe extern "C" fn(MppCtx, MppFrame, MppFrame) -> MPP_RET,
    pub isp_put_frame: unsafe extern "C" fn(MppCtx, MppFrame) -> MPP_RET,
    pub isp_get_frame: unsafe extern "C" fn(MppCtx, *mut MppFrame) -> MPP_RET,

    // Interfaz de tareas.
    pub poll: unsafe extern "C" fn(MppCtx, c_int, c_int) -> MPP_RET, // MppPortType, MppPollType
    pub dequeue: unsafe extern "C" fn(MppCtx, c_int, *mut MppTask) -> MPP_RET,
    pub enqueue: unsafe extern "C" fn(MppCtx, c_int, MppTask) -> MPP_RET,
    pub reset: unsafe extern "C" fn(MppCtx) -> MPP_RET,
    pub control: unsafe extern "C" fn(MppCtx, MpiCmd, MppParam) -> MPP_RET,
}

extern "C" {
    // rk_mpi.h — ciclo de vida del contexto.
    pub fn mpp_create(ctx: *mut MppCtx, mpi: *mut *mut MppApi) -> MPP_RET;
    pub fn mpp_init(ctx: MppCtx, ctx_type: c_int, coding: c_int) -> MPP_RET;
    pub fn mpp_destroy(ctx: MppCtx) -> MPP_RET;

    // rk_venc_cfg.h — configuración por claves string ("prep:width", "rc:*", …).
    pub fn mpp_enc_cfg_init(cfg: *mut MppEncCfg) -> MPP_RET;
    pub fn mpp_enc_cfg_deinit(cfg: MppEncCfg) -> MPP_RET;
    pub fn mpp_enc_cfg_set_s32(cfg: MppEncCfg, name: *const c_char, val: c_int) -> MPP_RET;
    pub fn mpp_enc_cfg_set_u32(cfg: MppEncCfg, name: *const c_char, val: u32) -> MPP_RET;
    pub fn mpp_enc_cfg_set_ptr(cfg: MppEncCfg, name: *const c_char, val: *mut c_void) -> MPP_RET;
    pub fn mpp_enc_cfg_get_s32(cfg: MppEncCfg, name: *const c_char, val: *mut c_int) -> MPP_RET;

    // mpp_buffer.h — grupos de buffers e importación de dma-buf.
    pub fn mpp_buffer_group_get(
        group: *mut MppBufferGroup,
        type_: c_int,
        mode: c_int,
        tag: *const c_char,
        caller: *const c_char,
    ) -> MPP_RET;
    pub fn mpp_buffer_group_put(group: MppBufferGroup) -> MPP_RET;
    pub fn mpp_buffer_import_with_tag(
        group: MppBufferGroup,
        info: *mut MppBufferInfo,
        buffer: *mut MppBuffer,
        tag: *const c_char,
        caller: *const c_char,
    ) -> MPP_RET;
    pub fn mpp_buffer_put_with_caller(buffer: MppBuffer, caller: *const c_char) -> MPP_RET;
    pub fn mpp_buffer_info_get_with_caller(
        buffer: MppBuffer,
        info: *mut MppBufferInfo,
        caller: *const c_char,
    ) -> MPP_RET;

    // mpp_packet.h — acceso a paquetes de salida.
    pub fn mpp_packet_get_data(packet: MppPacket) -> *mut c_void;
    pub fn mpp_packet_get_size(packet: MppPacket) -> usize;
    pub fn mpp_packet_get_eos(packet: MppPacket) -> u32;
    pub fn mpp_packet_deinit(packet: *mut MppPacket) -> MPP_RET;

    // mpp_frame.h — ensamblado del frame de entrada del encoder.
    pub fn mpp_frame_init(frame: *mut MppFrame) -> MPP_RET;
    pub fn mpp_frame_deinit(frame: *mut MppFrame) -> MPP_RET;
    pub fn mpp_frame_set_width(frame: MppFrame, width: u32);
    pub fn mpp_frame_set_height(frame: MppFrame, height: u32);
    pub fn mpp_frame_set_hor_stride(frame: MppFrame, hor_stride: u32);
    pub fn mpp_frame_set_ver_stride(frame: MppFrame, ver_stride: u32);
    pub fn mpp_frame_set_fmt(frame: MppFrame, fmt: c_int);
    pub fn mpp_frame_set_buf_size(frame: MppFrame, buf_size: usize);
    pub fn mpp_frame_set_buffer(frame: MppFrame, buffer: MppBuffer);
    pub fn mpp_frame_set_color_range(frame: MppFrame, color_range: c_int);
}
