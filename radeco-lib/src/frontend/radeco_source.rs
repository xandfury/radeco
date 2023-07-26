//! Defines the `Source` Trait.

use serde_json;

use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use r2papi::api_trait::R2Api;
use r2papi::structs::{
    FunctionInfo, LCCInfo, LEntryInfo, LExportInfo, LFlagInfo, LFunctionInfo, LImportInfo, LOpInfo,
    LRegInfo, LRelocInfo, LSectionInfo, LStringInfo, LSymbolInfo, LVarInfo,
};

#[derive(Debug)]
pub enum SourceErr {
    SrcErr(&'static str),
    OtherErr(Box<dyn Error>),
}

impl fmt::Display for SourceErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &SourceErr::SrcErr(s) => write!(f, "{}", s),
            &SourceErr::OtherErr(ref e) => write!(f, "{}", e),
        }
    }
}

impl<T: 'static + Error> From<T> for SourceErr {
    fn from(e: T) -> SourceErr {
        SourceErr::OtherErr(Box::new(e))
    }
}

// TODO: Split this up/compose this from more basic traits to avoid reimplementation
// as currently this is a re-implementation of r2papi
pub trait Source {
    fn functions(&self) -> Result<Vec<FunctionInfo>, SourceErr>;
    fn instructions_at(&self, _: u64) -> Result<Vec<LOpInfo>, SourceErr>;
    fn register_profile(&self) -> Result<LRegInfo, SourceErr>;
    fn flags(&self) -> Result<Vec<LFlagInfo>, SourceErr>;
    fn sections(&self) -> Result<Vec<LSectionInfo>, SourceErr>;
    fn symbols(&self) -> Result<Vec<LSymbolInfo>, SourceErr> {
        unimplemented!()
    }
    fn imports(&self) -> Result<Vec<LImportInfo>, SourceErr> {
        unimplemented!()
    }
    fn exports(&self) -> Result<Vec<LExportInfo>, SourceErr> {
        unimplemented!()
    }
    fn relocs(&self) -> Result<Vec<LRelocInfo>, SourceErr> {
        unimplemented!()
    }
    fn libraries(&self) -> Result<Vec<String>, SourceErr> {
        unimplemented!()
    }
    fn entrypoint(&self) -> Result<Vec<LEntryInfo>, SourceErr> {
        unimplemented!()
    }
    fn disassemble_function(&self, _name: &str) -> Result<Vec<LOpInfo>, SourceErr> {
        unimplemented!()
    }
    fn disassemble_n_bytes(&self, _n: u64, _at: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        unimplemented!()
    }
    fn disassemble_n_insts(&self, _n: u64, _at: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        unimplemented!()
    }
    fn locals_of(&self, _start_addr: u64) -> Result<Vec<LVarInfo>, SourceErr> {
        unimplemented!()
    }
    fn cc_info_of(&self, _start_addr: u64) -> Result<LCCInfo, SourceErr> {
        unimplemented!()
    }
    fn strings(&self, _data_only: bool) -> Result<Vec<LStringInfo>, SourceErr> {
        unimplemented!()
    }
    fn raw(&self, _cmd: String) -> Result<String, SourceErr> {
        unimplemented!()
    }

    fn send(&self, _: String) -> Result<(), SourceErr> {
        Ok(())
    }

    // Non essential / functions with default implementation.
    fn function_at(&self, address: u64) -> Result<FunctionInfo, SourceErr> {
        for f in self.functions()? {
            match f.offset {
                Some(off) if address == off => return Ok(f),
                _ => {}
            }
        }
        Err(SourceErr::SrcErr("No known function offset"))
    }

    fn function_named(&self, fn_name: &str) -> Result<FunctionInfo, SourceErr> {
        for f in self.functions()? {
            match f.name {
                Some(ref name) if name == fn_name => return Ok(f.clone()),
                _ => {}
            }
        }
        Err(SourceErr::SrcErr("No known function"))
    }

    fn instructions_at_fn(&self, fn_name: &str) -> Result<Vec<LOpInfo>, SourceErr> {
        let fn_ = self.function_named(fn_name)?;
        if let Some(offset) = fn_.offset {
            self.instructions_at(offset)
        } else {
            Err(SourceErr::SrcErr("No known function"))
        }
    }

    fn flag_at(&self, address: u64) -> Result<LFlagInfo, SourceErr> {
        for flag in self.flags()? {
            if flag.offset == address {
                return Ok(flag);
            }
        }
        Err(SourceErr::SrcErr("No known flag at offset"))
    }

    fn section_of(&self, address: u64) -> Result<LSectionInfo, SourceErr> {
        for s in self.sections()? {
            let addr = s.vaddr.expect("Invalid section");
            let size = s.size.expect("Invalid section size");
            if address >= addr && address < addr + size {
                return Ok(s);
            }
        }
        Err(SourceErr::SrcErr("No known section at addr"))
    }
}

// Cause R2Api requires borrowing mutably, while `Source` takes self which
// is immutable.
// The only problem with this is that r2pipe is not thread safe, therefore
// using `r2` from multiple threads will cause results to be inconsistent.
// However, this should not be a problem as `Rc` prevents concurrent accesses from multiple
// threads.
//
// TODO: For threading, this should be replaced by `RwLock<T>` or `Mutex<T>`
// But it is still up for discussion if this should be done at the r2pipe level rather than here.
// RwLock allows for multiple concurrent readers and one writer, however this will not improve
// the performance when compared to Mutex as every function takes `self` mutably (as it has to
// communicate with r2 which amounts to writing out to process pipe). Therefore, it'd make
// sense to have some sort of cached information so that concurrent reads can occur. This should
// be invalidated whenever some information is exported back to radare or some analysis is run on
// r2.
pub type WrappedR2Api<R> = Rc<RefCell<R>>;

// Implementation of `Source` trait for R2.
impl<R: R2Api> Source for WrappedR2Api<R> {
    fn functions(&self) -> Result<Vec<FunctionInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.fn_list()?)
    }

    fn instructions_at(&self, address: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        if let Ok(fn_info) = self.try_borrow_mut()?.function(&format!("{}", address)) {
            fn_info
                .ops
                .ok_or(SourceErr::SrcErr("No Instructions found"))
        } else {
            Err(SourceErr::SrcErr("No Instructions found"))
        }
    }

    fn register_profile(&self) -> Result<LRegInfo, SourceErr> {
        Ok(self.try_borrow_mut()?.reg_info()?)
    }

    fn flags(&self) -> Result<Vec<LFlagInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.flag_info()?)
    }

    fn sections(&self) -> Result<Vec<LSectionInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.sections()?)
    }

    fn symbols(&self) -> Result<Vec<LSymbolInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.symbols()?)
    }

    fn imports(&self) -> Result<Vec<LImportInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.imports()?)
    }

    fn exports(&self) -> Result<Vec<LExportInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.exports()?)
    }

    fn relocs(&self) -> Result<Vec<LRelocInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.relocs()?)
    }

    fn libraries(&self) -> Result<Vec<String>, SourceErr> {
        Ok(self.try_borrow_mut()?.libraries()?)
    }

    fn entrypoint(&self) -> Result<Vec<LEntryInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.entrypoint()?)
    }

    fn disassemble_n_bytes(&self, n: u64, at: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.disassemble_n_bytes(n, Some(at))?)
    }

    fn disassemble_n_insts(&self, n: u64, at: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.disassemble_n_insts(n, Some(at))?)
    }

    fn disassemble_function(&self, name: &str) -> Result<Vec<LOpInfo>, SourceErr> {
        Ok(self
            .try_borrow_mut()?
            .function(name)?
            .ops
            .unwrap_or(Vec::new()))
    }

    fn locals_of(&self, start_addr: u64) -> Result<Vec<LVarInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.locals_of(start_addr)?)
    }

    fn cc_info_of(&self, start_addr: u64) -> Result<LCCInfo, SourceErr> {
        Ok(self.try_borrow_mut()?.cc_info_of(start_addr)?)
    }

    fn strings(&self, data_only: bool) -> Result<Vec<LStringInfo>, SourceErr> {
        Ok(self.try_borrow_mut()?.strings(data_only)?)
    }

    fn raw(&self, cmd: String) -> Result<String, SourceErr> {
        Ok(self.try_borrow_mut()?.raw(cmd))
    }

    fn send(&self, s: String) -> Result<(), SourceErr> {
        let _ = self.try_borrow_mut()?.raw(s);
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// File source is used to load information from json files.
/// The files in the directory must be of the form "<base_name>_<suffix>.json". This means that the
/// files must have specific names.
///
/// To construct a FileSource painlessly (and save it for future use), use the `from::From` trait
/// implemented on `R2`.
pub struct FileSource {
    /// Directory where files should be loaded from.
    pub dir: String,
    /// Base name for the sample/example. suffixes will be appended based on what information is
    /// requested for. Hence, files in the directory must have specific names.
    pub base_name: String,
}

impl FileSource {
    fn read_file(&self, suffix: &str) -> Result<String, SourceErr> {
        let mut path = PathBuf::from(&self.dir);
        path.push(&format!("{}_{}.json", self.base_name, suffix));
        let mut f = File::open(path)?;
        let mut json_str = String::new();
        let _ = f.read_to_string(&mut json_str)?;
        Ok(json_str)
    }

    fn write_file(&mut self, suffix: &str, data: &str) {
        let mut path = PathBuf::from(&self.dir);
        path.push(&format!("{}_{}.json", self.base_name, suffix));
        let mut f = File::create(path).expect("Failed to open file");
        f.write_all(data.to_string().as_bytes())
            .expect("Failed to read file");
    }
}

mod suffix {
    pub const FUNCTION: &'static str = "function";
    pub const FUNCTION_INFO: &'static str = "fn_info";
    pub const INSTRUCTIONS: &'static str = "insts";
    pub const REGISTER: &'static str = "register_profile";
    pub const FLAG: &'static str = "flags";
    pub const SECTION: &'static str = "sections";
    pub const STRING: &'static str = "strings";
    pub const SYMBOL: &'static str = "symbols";
    pub const IMPORT: &'static str = "imports";
    pub const EXPORT: &'static str = "exports";
    pub const RELOC: &'static str = "relocs";
    pub const LIBRARY: &'static str = "libraries";
    pub const LOCAL: &'static str = "locals";
    pub const CCINFO: &'static str = "ccinfo";
    pub const ENTRY: &'static str = "entrypoint";
}

impl FileSource {
    pub fn open(f: &str) -> FileSource {
        let path = Path::new(f);
        let dir = path.parent().unwrap().to_str().unwrap();
        let base_name = path.file_name().unwrap().to_str().unwrap();
        FileSource {
            dir: dir.to_owned(),
            base_name: base_name.to_owned(),
        }
    }
}

impl Source for FileSource {
    fn functions(&self) -> Result<Vec<FunctionInfo>, SourceErr> {
        Ok(serde_json::from_str(
            &self.read_file(suffix::FUNCTION_INFO)?,
        )?)
    }

    fn instructions_at(&self, address: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        let suffix = format!("{}_{:#X}", suffix::INSTRUCTIONS, address);
        Ok(serde_json::from_str(&self.read_file(&suffix)?)?)
    }

    fn register_profile(&self) -> Result<LRegInfo, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::REGISTER)?)?)
    }

    fn flags(&self) -> Result<Vec<LFlagInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::FLAG)?)?)
    }

    fn sections(&self) -> Result<Vec<LSectionInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::SECTION)?)?)
    }

    fn symbols(&self) -> Result<Vec<LSymbolInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::SYMBOL)?)?)
    }

    fn imports(&self) -> Result<Vec<LImportInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::IMPORT)?)?)
    }

    fn exports(&self) -> Result<Vec<LExportInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::EXPORT)?)?)
    }

    fn relocs(&self) -> Result<Vec<LRelocInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::RELOC)?)?)
    }

    fn libraries(&self) -> Result<Vec<String>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::LIBRARY)?)?)
    }

    fn entrypoint(&self) -> Result<Vec<LEntryInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::ENTRY)?)?)
    }

    fn disassemble_n_bytes(&self, _n: u64, _at: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        Err(SourceErr::SrcErr(
            "`Source::disassemble_n_bytes` is not implemented",
        ))
    }

    fn disassemble_n_insts(&self, _n: u64, _at: u64) -> Result<Vec<LOpInfo>, SourceErr> {
        Err(SourceErr::SrcErr(
            "`Source::disassemble_n_insts` is not implemented",
        ))
    }

    fn disassemble_function(&self, name: &str) -> Result<Vec<LOpInfo>, SourceErr> {
        let suffix = format!("{}_{}", suffix::FUNCTION, name);
        let function_info: LFunctionInfo = serde_json::from_str(&self.read_file(&suffix)?)?;
        function_info
            .ops
            .ok_or(SourceErr::SrcErr("LFunctionInfo::ops is `None`"))
    }

    fn locals_of(&self, start_addr: u64) -> Result<Vec<LVarInfo>, SourceErr> {
        let suffix = format!("{}_{}", suffix::LOCAL, start_addr);
        Ok(serde_json::from_str(&self.read_file(&suffix)?)?)
    }

    fn cc_info_of(&self, start_addr: u64) -> Result<LCCInfo, SourceErr> {
        let suffix = format!("{}_{}", suffix::CCINFO, start_addr);
        Ok(serde_json::from_str(&self.read_file(&suffix)?)?)
    }

    fn strings(&self, _data_only: bool) -> Result<Vec<LStringInfo>, SourceErr> {
        Ok(serde_json::from_str(&self.read_file(suffix::STRING)?)?)
    }

    fn raw(&self, _cmd: String) -> Result<String, SourceErr> {
        Err(SourceErr::SrcErr("`Source::raw` is not implemented"))
    }
}

impl<R: R2Api> From<WrappedR2Api<R>> for FileSource {
    fn from(r2: WrappedR2Api<R>) -> FileSource {
        let bin_info = r2.borrow_mut().bin_info().expect("Failed to load bin_info");
        let fname = bin_info.core.unwrap().file.unwrap();
        let fname = Path::new(&fname).file_stem().unwrap();
        let mut dir = PathBuf::from(".");
        dir.push(&fname);
        fs::create_dir_all(&dir).expect("Failed to create directory");
        let mut fsource = FileSource {
            dir: dir.to_str().unwrap().to_owned(),
            base_name: fname.to_str().unwrap().to_owned(),
        };

        {
            let fns = r2.functions().expect("Failed to load function info");
            {
                let json_str = serde_json::to_string(&fns).expect("Failed to encode to json");
                fsource.write_file(suffix::FUNCTION_INFO, &json_str);
            }

            for f in fns {
                {
                    let result = r2
                        .instructions_at(f.offset.unwrap())
                        .expect("Failed to load instructions");
                    let json_str =
                        serde_json::to_string(&result).expect("Failed to encode to json");
                    let suffix = format!("{}_{:#X}", suffix::INSTRUCTIONS, f.offset.unwrap());
                    fsource.write_file(&suffix, &json_str)
                }

                {
                    let name = f.name.clone().unwrap();
                    let result = r2
                        .disassemble_function(&name)
                        .map(|ops| LFunctionInfo {
                            addr: f.offset,
                            name: f.name.clone(),
                            ops: Some(ops),
                            size: f.size,
                        })
                        .expect(&format!("Failed to load instructions @ {}", name));
                    let json_str =
                        serde_json::to_string(&result).expect("Failed to encode to json");
                    let suffix = format!("{}_{}", suffix::FUNCTION, name);
                    fsource.write_file(&suffix, &json_str)
                }

                {
                    let result = r2
                        .locals_of(f.offset.unwrap())
                        .expect("Failed to load locals");
                    let json_str =
                        serde_json::to_string(&result).expect("Failed to encode to json");
                    let suffix = format!("{}_{}", suffix::LOCAL, f.offset.unwrap());
                    fsource.write_file(&suffix, &json_str)
                }

                {
                    let result = r2
                        .cc_info_of(f.offset.unwrap())
                        .expect("Failed to load cc info");
                    let json_str =
                        serde_json::to_string(&result).expect("Failed to encode to json");
                    let suffix = format!("{}_{}", suffix::CCINFO, f.offset.unwrap());
                    fsource.write_file(&suffix, &json_str)
                }
            }

            {
                let reg = r2.register_profile().expect("Failed to load register info");
                let json_str = serde_json::to_string(&reg).expect("Failed to encode to json");
                fsource.write_file(suffix::REGISTER, &json_str);
            }

            {
                let flags = r2.flags().expect("Failed to load flag info");
                let json_str = serde_json::to_string(&flags).expect("Failed to encode to json");
                fsource.write_file(suffix::FLAG, &json_str);
            }

            {
                let sections = r2.sections().expect("Failed to load section map");
                let json_str = serde_json::to_string(&sections).expect("Failed to encode to json");
                fsource.write_file(suffix::SECTION, &json_str);
            }

            {
                let strings = r2
                    .borrow_mut()
                    .strings(true)
                    .expect("Unable to load String info from r2");
                let json_str = serde_json::to_string(&strings).expect("Failed to encode to json");
                fsource.write_file(suffix::STRING, &json_str);
            }

            {
                let imports = r2
                    .borrow_mut()
                    .imports()
                    .expect("Unable to load import info from r2");
                let json_str = serde_json::to_string(&imports).expect("Failed to encode to json");
                fsource.write_file(suffix::IMPORT, &json_str);
            }

            // FIXME: Error occurs during `r2.borrow_mut().exports()`
            // {
            //     let exports = r2.borrow_mut().exports().expect("Unable to load export info from r2");
            //     let json_str = serde_json::to_string(&exports).expect("Failed to encode to json");
            //     fsource.write_file(suffix::EXPORT, &json_str);
            // }

            {
                let relocs = r2
                    .borrow_mut()
                    .relocs()
                    .expect("Unable to load reloc info from r2");
                let json_str = serde_json::to_string(&relocs).expect("Failed to encode to json");
                fsource.write_file(suffix::RELOC, &json_str);
            }

            {
                let libraries = r2
                    .borrow_mut()
                    .libraries()
                    .expect("Unable to load library info from r2");
                let json_str = serde_json::to_string(&libraries).expect("Failed to encode to json");
                fsource.write_file(suffix::LIBRARY, &json_str);
            }

            {
                let entry = r2
                    .borrow_mut()
                    .entrypoint()
                    .expect("Unable to load entry info from r2");
                let json_str = serde_json::to_string(&entry).expect("Failed to encode to json");
                fsource.write_file(suffix::ENTRY, &json_str);
            }
        }

        fsource
    }
}

#[cfg(test)]
mod test {

    use crate::frontend::radeco_containers::*;
    use crate::frontend::radeco_source::*;
    use std::rc::Rc;

    #[test]
    fn file_source_test() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_files/bin1_filesource/bin1");
        let source = FileSource::open(path.to_str().unwrap());
        ProjectLoader::new().source(Rc::new(source)).load();
    }
}
