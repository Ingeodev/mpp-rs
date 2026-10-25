# mpp-rs

Bindings Rust de **Rockchip MPP** (`librockchip_mpp`, SDK open-source
[rockchip-linux/mpp](https://github.com/rockchip-linux/mpp)) para el
ecosistema **Afora**. Espejo de [`rga-rs`](../rga-rs): `mpp-sys` (FFI cruda)
+ `mpp` (wrapper seguro).

## Ecosistema Afora

| Repo | Rol |
|---|---|
| `afora-imx662-camera-adapter-rs` | Captura V4L2 → publica frames dma-buf (bloque E) |
| `rga-rs` | Conversión de color por hardware (UYVY→NV12) |
| **`mpp-rs`** | **Encoder hardware MPP (H.264/H.265) — este repo** |
| `afora-live-svc-rs` | Servicio live: RGA → MPP → RTSP (cliente #2) |
| cliente storage (planeado) | MP4 H.265 — reutilizará este crate (cliente #3) |
| `afora-mediamtx` | Media server RTSP/WHEP |
| `afora-vision` | Panel Angular con player WHEP |

## Estructura

```
mpp-sys/   FFI manual (~15 símbolos), sin bindgen; enlaza vía pkg-config
           (rockchip_mpp.pc) con override MPP_DIR (default $HOME/.local)
mpp/       Wrapper seguro: MppEncoder (ciclo de vida + config), BufferGroup /
           ImportedBuffer (import de dma-buf por fd), EncoderConfig, Error
```

## Estado v0

- ✅ `mpp_create` / `mpp_init(ENC)` / `mpp_destroy`, `mpp_enc_cfg_*`
  (claves string: `prep:width`, `prep:height`, …), grupos de buffers e
  importación de dma-buf por fd (`MPP_BUFFER_TYPE_EXT_DMA`).
- ⏳ `encode` (ensamblado de `MppFrame`, `MPP_ENC_GET_HDR_SYNC` y extracción
  de packets): fase **F6a** de `afora-live-svc-rs`; los valores `MpiCmd` se
  generan con un probe C contra `rk_mpi_cmd.h` (no se hardcodean).

## Dependencias de la placa

1. SDK MPP instalado en `~/.local` (`librockchip_mpp.so` + headers) — ver
   README de la raíz del workspace, sección P0-a.
2. Acceso no-root a `/dev/mpp_service` (regla udev `99-afora-mpp.rules`) —
   README de la raíz, P0-b.
3. Acceso no-root a `/dev/dma_heap/*` (regla udev `99-afora-dma-heap.rules`)
   — README de la raíz, P0-c. Sin esto, `mpp_init` falla aunque el nodo
   `mpp_service` sea accesible: el HAL del encoder aloca sus buffers
   internos por ION→dma-heap.
4. Compilar siempre con `-j 2` en la placa (3.8 GB RAM).

Prueba de hardware (placa):
`cargo test -j 2 --test hw_encoder -- --include-ignored`

## Convenciones

- Arquitectura limpia por capas en los clientes; aquí, como en `rga-rs`,
  solo hay bindings de hardware (`sys` crudo + wrapper seguro).
- Un caso de uso por archivo; sin ficheros extensos.
- Toda la FFI `unsafe` queda aislada en `mpp-sys` y los detalles
  `unsafe` del wrapper están justificados con `// SAFETY:`.
