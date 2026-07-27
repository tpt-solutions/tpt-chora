pub struct WebGpuBootstrap {
    canvas_width: u32,
    canvas_height: u32,
    wasm_url: String,
    shader_urls: Vec<String>,
    wasm_binary_size: Option<usize>,
    _initialized: bool,
}

impl WebGpuBootstrap {
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            canvas_width,
            canvas_height,
            wasm_url: String::new(),
            shader_urls: Vec::new(),
            wasm_binary_size: None,
            _initialized: false,
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

    pub fn with_wasm_binary_size(mut self, size: usize) -> Self {
        self.wasm_binary_size = Some(size);
        self
    }

    pub fn generate_bootstrap_script(&self) -> String {
        let shader_fetches: String = self
            .shader_urls
            .iter()
            .enumerate()
            .map(|(i, url)| {
                format!(
                    "    const shaderResponse_{i} = await fetch(\"{url}\");\n    \
                     const shaderCode_{i} = await shaderResponse_{i}.text();\n"
                )
            })
            .collect();

        let shader_modules: String = if !self.shader_urls.is_empty() {
            format!(
                "    const shaderModules = [{}];\n",
                self.shader_urls
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("shaderCode_{i}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            "    const shaderModules = [];\n".to_string()
        };

        format!(
            r#"// tpt-chora WebGPU Bootstrap
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

{shader_fetches}
{shader_modules}
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
            shader_fetches = shader_fetches,
            shader_modules = shader_modules,
        )
    }

    pub fn bootstrap_size_bytes(&self) -> usize {
        self.generate_bootstrap_script().len()
    }

    pub fn wasm_binary_size_bytes(&self) -> Option<usize> {
        self.wasm_binary_size
    }
}
