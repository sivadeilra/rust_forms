use anyhow::{Result, bail};
use ms_pdb::dbi::ModuleInfoFixed;
use std::collections::HashMap;
use sync_file::ReadAt;
use zerocopy::IntoBytes;

use ms_pdb::Pdb;

/// Contains knowledge ("ken") about a given PDB.
pub(crate) struct PdbKen {
    pub pdb: Box<Pdb>,

    pub modules: Vec<ModuleKen>,

    /// Contains cached symbol data for modules.
    ///
    /// Key is module index
    /// Value is the symbol stream, including 4-byte header.
    pub module_symbols: HashMap<usize, Vec<u32>>,
}

pub(crate) struct ModuleKen {
    pub module_name: String,
    pub obj_file: String,
    pub header: ModuleInfoFixed,
}

impl PdbKen {
    pub fn new(pdb: Box<Pdb>) -> Result<Self> {
        let modules = pdb.modules()?;
        let mut modules_ken: Vec<ModuleKen> = Vec::with_capacity(modules.iter().count());

        for module in modules.iter() {
            modules_ken.push(ModuleKen {
                module_name: module.module_name().to_string(),
                obj_file: module.obj_file().to_string(),
                header: module.header.clone(),
            });
        }

        Ok(Self {
            pdb,
            modules: modules_ken,
            module_symbols: Default::default(),
        })
    }

    pub fn load_module_symbols(&mut self, module_index: usize) -> Result<()> {
        if self.module_symbols.contains_key(&module_index) {
            return Ok(());
        }

        let Some(module) = self.modules.get(module_index) else {
            bail!("Invalid module index");
        };

        let Some(stream) = module.header.stream() else {
            self.module_symbols.insert(module_index, Vec::new());
            return Ok(());
        };

        let len_u32 = module.header.sym_byte_size.get() as usize / size_of::<u32>();
        let mut symbol_data: Vec<u32> = vec![0; len_u32];

        let sr = self.pdb.get_stream_reader(stream)?;
        sr.read_exact_at(symbol_data.as_mut_bytes(), 0)?;

        self.module_symbols.insert(module_index, symbol_data);
        Ok(())
    }

    pub fn get_module_symbols(&self, module_index: usize) -> Option<&[u32]> {
        self.module_symbols.get(&module_index).map(|s| s.as_slice())
    }
}
