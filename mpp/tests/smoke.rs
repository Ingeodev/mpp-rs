//! Test de humo: verifica que el binario enlaza contra `librockchip_mpp`
//! y que `mpp_enc_cfg_init/deinit` funcionan (no toca `/dev/mpp_service`).

use mpp::ffi;

#[test]
fn cfg_init_deinit_enlaza() {
    unsafe {
        let mut cfg: ffi::MppEncCfg = std::ptr::null_mut();
        assert_eq!(ffi::mpp_enc_cfg_init(&mut cfg), ffi::MPP_OK);
        assert!(!cfg.is_null());

        // Clave base verificada contra la fuente del SDK (mpp_enc_impl.c):
        // round-trip set → get por clave string.
        assert_eq!(
            ffi::mpp_enc_cfg_set_s32(cfg, c"prep:width".as_ptr(), 1280),
            ffi::MPP_OK
        );
        let mut w: std::os::raw::c_int = 0;
        assert_eq!(
            ffi::mpp_enc_cfg_get_s32(cfg, c"prep:width".as_ptr(), &mut w),
            ffi::MPP_OK
        );
        assert_eq!(w, 1280);

        assert_eq!(ffi::mpp_enc_cfg_deinit(cfg), ffi::MPP_OK);
    }
}
