use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=MPP_DIR");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    // Asegurar que pkg-config vea el .pc del SDK MPP instalado localmente
    // (make install → $HOME/.local; override con MPP_DIR).
    let mpp_dir = env::var("MPP_DIR")
        .unwrap_or_else(|_| format!("{}/.local", env::var("HOME").expect("HOME")));
    let pc_path = format!("{mpp_dir}/lib/pkgconfig");
    let existing = env::var("PKG_CONFIG_PATH").unwrap_or_default();
    if !existing.split(':').any(|p| p == pc_path) {
        let combined = if existing.is_empty() {
            pc_path
        } else {
            format!("{existing}:{pc_path}")
        };
        // SAFETY: build script single-threaded; solo añade la ruta del SDK.
        env::set_var("PKG_CONFIG_PATH", combined);
    }

    // Probe + enlazado (emite cargo:rustc-link-lib/search automáticamente;
    // el crate declara links = "rockchip_mpp").
    pkg_config::Config::new()
        .atleast_version("1.0")
        .probe("rockchip_mpp")
        .expect(
            "No se encontró rockchip_mpp (rockchip_mpp.pc). ¿Está instalado el \
             SDK MPP? Ver README de la raíz del workspace (sección P0-a). \
             Override: MPP_DIR=<prefijo>.",
        );

    // NOTA: el rpath de runtime para librockchip_mpp.so.1 se define en
    // .cargo/config.toml del workspace (rustc-link-arg de una dependencia
    // no se propaga a los binarios/tests de los crates cliente).
}
