//! Test de hardware (#[ignore] por defecto: fallaría en máquinas sin el
//! SoC). Verifica P0 completo: acceso no-root a `/dev/mpp_service` y la
//! creación real de un canal encoder.
//!
//! Ejecutar en la placa: `cargo test -j 2 --test hw_encoder -- --ignored`

use mpp::{Coding, EncoderConfig, MppEncoder};

#[test]
fn crea_y_destruye_encoder_real() {
    let cfg = EncoderConfig {
        width: 1280,
        height: 720,
        coding: Coding::Avc,
        fps_in: 6,
        fps_out: 6,
        gop: 12,
        bps: (1_000_000, 4_000_000),
    };
    let enc = MppEncoder::new(cfg).expect(
        "mpp_create/mpp_init falló. Causas típicas (README de la raíz, P0): \
         (b) permisos de /dev/mpp_service (crw-rw---- root video) o \
         (c) permisos de /dev/dma_heap/* — MPP aloca sus buffers internos \
         por ION→dma-heap",
    );
    assert_eq!(enc.config().width, 1280);
    // Drop → mpp_destroy sin leak ni hang.
}
