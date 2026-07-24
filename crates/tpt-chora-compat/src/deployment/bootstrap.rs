pub struct WebGpuBootstrap {
    canvas_width: u32,
    canvas_height: u32,
    wasm_url: String,
    shader_urls: Vec<String>,
    initialized: bool,
}

impl WebGpuBootstrap {
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            canvas_width,
            canvas_height,
            wasm_url: String::new(),
            shader_urls: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_wasm_url(mut self, url: String) -> Self {
        self.wasm_url = url;
        self
    }

    pub fn with_shader_urls(mut self, urls: Vec<String>) -> Self {
        self.shader_urls = urls;
        self
    }

    pub fn generate_bootstrap_script(&self) -> String {
        format!(
            r#"// tpt-chora WebGPU Bootstrap (~500KB target)
(async function() {{
    "use strict";

    const canvas = document.createElement("canvas");
    canvas.width = {width};
    canvas.height = {height};
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    document.body.appendChild(canvas);
    document.body.style.margin = "0";
    document.body.style.overflow = "hidden";

    const adapter = await navigator.gpu.requestAdapter({{ powerPreference: "high-performance" }});
    if (!adapter) {{
        console.error("tpt-chora: No WebGPU adapter found");
        return;
    }}

    const device = await adapter.requestDevice();
    const context = canvas.getContext("webgpu");
    const format = navigator.gpu.getPreferredCanvasFormat();

    context.configure({{
        device,
        format,
        alphaMode: "premultiplied",
    }});

    const wasmResponse = await fetch("{wasm_url}");
    const wasmBytes = await wasmResponse.arrayBuffer();
    const {{ instance }} = await WebAssembly.instantiate(wasmBytes, {{
        env: {{
            canvas_width: {width},
            canvas_height: {height},
        }},
    }});

    console.log("tpt-chora: Runtime initialized ({width}x{height})");
    console.log("tpt-chora: DOM bypassed, rendering via WebGPU");
}})();"#,
            width = self.canvas_width,
            height = self.canvas_height,
            wasm_url = self.wasm_url,
        )
    }

    pub fn bootstrap_size_bytes(&self) -> usize {
        self.generate_bootstrap_script().len()
    }
}
