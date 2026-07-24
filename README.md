# tpt-chora

**Proof-native, GPU-accelerated presentation, interaction, and media runtime
for the TPT ecosystem.**

`tpt-chora` entirely replaces the traditional browser rendering pipeline: it
natively handles 2D/3D UI rendering, complex typography, spatial computing
(XR), hardware-accelerated media, multi-modal input, and OS-level
accessibility. It operates strictly as the "receptacle," reading zero-copy
memory from [`tpt-archon`](https://github.com/tpt-solutions/tpt-archon)
(the substrate), consuming proven geometry and semantics from
[`tpt-eidos`](https://github.com/tpt-solutions/tpt-eidos) (form/layout), and
executing stateful interactions via
[`tpt-telos`](https://github.com/tpt-solutions/tpt-telos) (logic/state).

See `spec.txt` for the full design document and `todo.md` for the phased
build roadmap.

## Status

Early build-out. **Phase 1 (Core Rendering Engine / "The Canvas")** is in
progress: a wgpu-backed, frame-scoped render graph; GPU-compute-shader
cubic-Bezier vector tessellation; and a color-grading post-process pass.

## Workspace crates

| Crate | Purpose |
| --- | --- |
| `tpt-chora-render` | The Core Rendering Engine (render graph, vector tessellation, post-processing) |

## Try it

```sh
cargo run -p tpt-chora-render --example triangle_and_path
```

Renders a triangle and a GPU-tessellated vector path (a circle built from
four cubic Beziers) through the render graph and post-process pass to an
off-screen target, then writes the result to
`crates/tpt-chora-render/examples/output/triangle_and_path.png`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
