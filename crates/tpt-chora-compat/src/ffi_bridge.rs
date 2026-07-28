use wasmtime::{Engine, Instance, Linker, Module, Store, Val, ValType};

pub struct FfiBridge {
    engine: Engine,
    modules: Vec<LoadedModule>,
}

struct LoadedModule {
    info: WasmModule,
    store: Store<()>,
    instance: Instance,
}

#[derive(Debug, Clone)]
pub struct WasmModule {
    pub id: u64,
    pub name: String,
    pub memory_size: usize,
    pub exports: Vec<String>,
}

impl FfiBridge {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
            modules: Vec::new(),
        }
    }

    pub fn register_module(
        &mut self,
        name: String,
        data: &[u8],
    ) -> Result<u64, crate::CompatError> {
        let module = Module::new(&self.engine, data)
            .map_err(|e| crate::CompatError::WasmLoadFailed(e.to_string()))?;

        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| crate::CompatError::WasmLoadFailed(e.to_string()))?;

        let exports: Vec<String> = module.exports().map(|e| e.name().to_string()).collect();
        let memory_size = instance
            .get_memory(&mut store, "memory")
            .map(|m| m.data_size(&store))
            .unwrap_or(0);

        let id = self.modules.len() as u64;
        self.modules.push(LoadedModule {
            info: WasmModule {
                id,
                name,
                memory_size,
                exports,
            },
            store,
            instance,
        });
        Ok(id)
    }

    /// Calls an exported function, decoding `args` as a tightly packed,
    /// little-endian sequence of scalar values matching the function's
    /// param types, and returning its results packed the same way.
    pub fn call_function(
        &mut self,
        module_id: u64,
        function: &str,
        args: &[u8],
    ) -> Result<Vec<u8>, crate::CompatError> {
        let loaded = self.modules.get_mut(module_id as usize).ok_or_else(|| {
            crate::CompatError::FfiError(format!("module {} not found", module_id))
        })?;

        let func = loaded
            .instance
            .get_func(&mut loaded.store, function)
            .ok_or_else(|| {
                crate::CompatError::FfiError(format!(
                    "function '{}' not exported by module '{}'",
                    function, loaded.info.name
                ))
            })?;

        let ty = func.ty(&loaded.store);

        let mut params = Vec::with_capacity(ty.params().len());
        let mut offset = 0usize;
        for param_ty in ty.params() {
            let val = read_val(&param_ty, args, &mut offset).map_err(|e| {
                crate::CompatError::FfiError(format!("decoding args for '{}': {}", function, e))
            })?;
            params.push(val);
        }
        if offset != args.len() {
            return Err(crate::CompatError::FfiError(format!(
                "'{}' expects {} bytes of packed args, got {}",
                function,
                offset,
                args.len()
            )));
        }

        let mut results = vec![Val::I32(0); ty.results().len()];
        func.call(&mut loaded.store, &params, &mut results)
            .map_err(|e| {
                crate::CompatError::FfiError(format!("call to '{}' failed: {}", function, e))
            })?;

        let mut out = Vec::new();
        for result in &results {
            write_val(result, &mut out);
        }
        Ok(out)
    }

    pub fn get_module(&self, id: u64) -> Option<&WasmModule> {
        self.modules.get(id as usize).map(|m| &m.info)
    }

    pub fn modules(&self) -> Vec<&WasmModule> {
        self.modules.iter().map(|m| &m.info).collect()
    }
}

fn read_val(ty: &ValType, args: &[u8], offset: &mut usize) -> Result<Val, String> {
    let size = match ty {
        ValType::I32 | ValType::F32 => 4,
        ValType::I64 | ValType::F64 => 8,
        other => return Err(format!("unsupported param type {:?}", other)),
    };
    if *offset + size > args.len() {
        return Err("args buffer too short".into());
    }
    let bytes = &args[*offset..*offset + size];
    *offset += size;
    Ok(match ty {
        ValType::I32 => Val::I32(i32::from_le_bytes(bytes.try_into().unwrap())),
        ValType::I64 => Val::I64(i64::from_le_bytes(bytes.try_into().unwrap())),
        ValType::F32 => Val::F32(u32::from_le_bytes(bytes.try_into().unwrap())),
        ValType::F64 => Val::F64(u64::from_le_bytes(bytes.try_into().unwrap())),
        _ => unreachable!(),
    })
}

fn write_val(val: &Val, out: &mut Vec<u8>) {
    match val {
        Val::I32(v) => out.extend_from_slice(&v.to_le_bytes()),
        Val::I64(v) => out.extend_from_slice(&v.to_le_bytes()),
        Val::F32(bits) => out.extend_from_slice(&bits.to_le_bytes()),
        Val::F64(bits) => out.extend_from_slice(&bits.to_le_bytes()),
        _ => {}
    }
}

impl Default for FfiBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (module
    //   (func (export "add") (param i32 i32) (result i32)
    //     local.get 0
    //     local.get 1
    //     i32.add))
    const ADD_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f,
        0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
        0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
    ];

    #[test]
    fn register_module_reads_exports() {
        let mut bridge = FfiBridge::new();
        let id = bridge.register_module("add-mod".into(), ADD_WASM).unwrap();
        let module = bridge.get_module(id).unwrap();
        assert_eq!(module.name, "add-mod");
        assert_eq!(module.exports, vec!["add".to_string()]);
    }

    #[test]
    fn register_module_rejects_invalid_wasm() {
        let mut bridge = FfiBridge::new();
        let err = bridge.register_module("bad".into(), &[0, 1, 2, 3]);
        assert!(err.is_err());
    }

    #[test]
    fn call_function_executes_real_wasm() {
        let mut bridge = FfiBridge::new();
        let id = bridge.register_module("add-mod".into(), ADD_WASM).unwrap();

        let mut args = Vec::new();
        args.extend_from_slice(&3i32.to_le_bytes());
        args.extend_from_slice(&4i32.to_le_bytes());

        let result = bridge.call_function(id, "add", &args).unwrap();
        assert_eq!(i32::from_le_bytes(result.try_into().unwrap()), 7);
    }

    #[test]
    fn call_function_unknown_export_errors() {
        let mut bridge = FfiBridge::new();
        let id = bridge.register_module("add-mod".into(), ADD_WASM).unwrap();
        assert!(bridge.call_function(id, "missing", &[]).is_err());
    }

    #[test]
    fn call_function_mismatched_arg_length_errors() {
        let mut bridge = FfiBridge::new();
        let id = bridge.register_module("add-mod".into(), ADD_WASM).unwrap();
        assert!(bridge.call_function(id, "add", &[1, 2, 3]).is_err());
    }
}
