pub struct FfiBridge {
    modules: Vec<WasmModule>,
    function_sigs: std::collections::HashMap<(u64, String), Vec<u8>>,
}

pub struct WasmModule {
    pub id: u64,
    pub name: String,
    pub memory_size: usize,
    pub exports: Vec<String>,
}

const WASM_MAGIC_NUMBERS: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

impl FfiBridge {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            function_sigs: std::collections::HashMap::new(),
        }
    }

    pub fn register_module(
        &mut self,
        name: String,
        data: &[u8],
    ) -> Result<u64, crate::CompatError> {
        if data.len() < 8 {
            return Err(crate::CompatError::WasmLoadFailed(
                "data too small for Wasm module".into(),
            ));
        }

        if data[..4] != WASM_MAGIC_NUMBERS {
            return Err(crate::CompatError::WasmLoadFailed(
                "invalid Wasm magic number".into(),
            ));
        }

        let mut memory_size = 0usize;
        let mut exports = Vec::new();

        let mut pos = 8;
        while pos + 8 <= data.len() {
            let section_id = data[pos];
            pos += 1;

            if pos + 4 > data.len() {
                break;
            }
            let section_size =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            let section_end = (pos + section_size).min(data.len());

            match section_id {
                5 => {
                    while pos + 1 < section_end {
                        let name_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
                        pos += 2;
                        if pos + name_len <= section_end {
                            let name =
                                String::from_utf8_lossy(&data[pos..pos + name_len]).to_string();
                            exports.push(name);
                            pos += name_len;
                        } else {
                            break;
                        }
                    }
                }
                6 => {
                    while pos < section_end {
                        let init = data[pos];
                        if init == 1 && pos + 5 < section_end {
                            memory_size = u32::from_le_bytes([
                                data[pos + 1],
                                data[pos + 2],
                                data[pos + 3],
                                data[pos + 4],
                            ]) as usize;
                        }
                        pos += 5;
                    }
                }
                _ => {}
            }

            pos = section_end;
        }

        let id = self.modules.len() as u64;
        self.modules.push(WasmModule {
            id,
            name,
            memory_size,
            exports,
        });
        Ok(id)
    }

    pub fn call_function(
        &self,
        module_id: u64,
        function: &str,
        args: &[u8],
    ) -> Result<Vec<u8>, crate::CompatError> {
        let module = self.modules.get(module_id as usize).ok_or_else(|| {
            crate::CompatError::FfiError(format!("module {} not found", module_id))
        })?;

        if !module.exports.iter().any(|e| e == function) {
            return Err(crate::CompatError::FfiError(format!(
                "function '{}' not exported by module '{}'",
                function, module.name
            )));
        }

        if let Some(sig) = self.function_sigs.get(&(module_id, function.to_string())) {
            return Ok(sig.clone());
        }

        Ok(args.to_vec())
    }

    pub fn get_module(&self, id: u64) -> Option<&WasmModule> {
        self.modules.get(id as usize)
    }

    pub fn modules(&self) -> &[WasmModule] {
        &self.modules
    }
}
