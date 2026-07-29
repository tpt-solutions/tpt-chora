pub struct WebGpuBootstrap {
    canvas_width: u32,
    canvas_height: u32,
    wasm_url: String,
    shader_urls: Vec<String>,
    wasm_binary_size: Option<usize>,
    _initialized: bool,
}

/// Escapes a string for safe embedding inside a double-quoted JS string
/// literal in the generated bootstrap script. `wasm_url`/`shader_urls` are
/// public setters (`with_wasm_url`/`with_shader_urls`), so anything other
/// than a hardcoded literal could otherwise inject arbitrary JS/HTML into
/// the page this script generates.
fn escape_js_string_literal(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '<' => escaped.push_str("\\u003C"),
            '>' => escaped.push_str("\\u003E"),
            _ => escaped.push(c),
        }
    }
    escaped
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
                let url = escape_js_string_literal(url);
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
            wasm_url = escape_js_string_literal(&self.wasm_url),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        let escaped = escape_js_string_literal("a\"b\\c\nd");
        assert_eq!(escaped, "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn escapes_angle_brackets_to_prevent_script_breakout() {
        let escaped = escape_js_string_literal("</script><script>alert(1)</script>");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
    }

    #[test]
    fn malicious_wasm_url_cannot_break_out_of_the_string_literal() {
        // If the closing `"` in this payload weren't escaped, the
        // generated `fetch("...")` call would have its string argument
        // closed early, leaving `fetch("");` behind followed by the
        // injected statement executing outside any string.
        let bootstrap = WebGpuBootstrap::new(100, 100)
            .with_wasm_url("\");alert(document.cookie);//".to_string());
        let script = bootstrap.generate_bootstrap_script();
        assert!(
            !script.contains("fetch(\"\");"),
            "quote was not escaped, closing the fetch() string literal early:\n{script}"
        );
        assert!(script.contains("\\\");alert(document.cookie);//"));
    }

    #[test]
    fn malicious_shader_url_cannot_break_out_of_the_string_literal() {
        let bootstrap = WebGpuBootstrap::new(100, 100)
            .with_wasm_url("https://cdn.example.com/chora.wasm".to_string())
            .with_shader_urls(vec!["\";fetch('https://evil.example');//".to_string()]);
        let script = bootstrap.generate_bootstrap_script();
        assert!(
            !script.contains("fetch(\"\");"),
            "quote was not escaped, closing the fetch() string literal early:\n{script}"
        );
    }

    #[test]
    fn benign_url_round_trips_unescaped() {
        let bootstrap = WebGpuBootstrap::new(100, 100)
            .with_wasm_url("https://cdn.example.com/chora.wasm".to_string());
        let script = bootstrap.generate_bootstrap_script();
        assert!(script.contains("https://cdn.example.com/chora.wasm"));
    }
}
