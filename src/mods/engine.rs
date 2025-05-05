use std::fmt;

use anyhow::*;

#[derive(Clone)]
pub(crate) struct Engine(wasmtime::Engine);

impl Default for Engine {
    fn default() -> Self {
        let mut config = wasmtime::Config::new();

        config
            .cache_config_load_default()
            .expect("Failed to load cache config");
        config.parallel_compilation(true);
        config.wasm_custom_page_sizes(true);

        // Enable pooling
        // https://docs.wasmtime.dev/examples-fast-instantiation.html
        let mut pool = wasmtime::PoolingAllocationConfig::new();
        pool.total_memories(100);
        pool.max_memory_size(1 << 31); // 2 GiB
        pool.total_tables(100);
        pool.table_elements(5000);
        pool.total_core_instances(100);
        config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(pool));

        let engine = wasmtime::Engine::new(&config).unwrap();

        Self(engine)
    }
}

pub(crate) struct Module(wasmtime::Module);

impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.0.name().unwrap_or("unnamed");
        f.debug_struct("Module").field("name", &name).finish()
    }
}

impl Module {
    pub fn new(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Self> {
        let module = wasmtime::Module::new(&engine.0, bytes)?;
        Ok(Self(module))
    }
}
