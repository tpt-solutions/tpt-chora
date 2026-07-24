pub struct FfiBridge {
    modules: Vec<WasmModule>,
}

pub struct WasmModule {
    pub id: u64,
    pub name: String,
    pub memory_size: usize,
    pub exports: Vec<String>,
}

impl FfiBridge {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn register_module(&mut self, name: String, data: &[u8]) -> Result<u64, crate::CompatError> {
        let id = self.modules.len() as u64;
        self.modules.push(WasmModule {
            id,
            name,
            memory_size: data.len(),
            exports: Vec::new(),
        });
        Ok(id)
    }

    pub fn call_function(
        &self,
        _module_id: u64,
        _function: &str,
        _args: &[u8],
    ) -> Result<Vec<u8>, crate::CompatError> {
        Ok(Vec::new())
    }

    pub fn get_module(&self, id: u64) -> Option<&WasmModule> {
        self.modules.get(id as usize)
    }

    pub fn modules(&self) -> &[WasmModule] {
        &self.modules
    }
}
