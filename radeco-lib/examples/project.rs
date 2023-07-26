// Examples to illustrate project loading

extern crate r2papi;
extern crate r2pipe;
extern crate radeco_lib;

use r2papi::api_trait::R2Api;
use r2pipe::R2;
use radeco_lib::frontend::radeco_containers::{FunctionLoader, ModuleLoader, ProjectLoader};
use radeco_lib::frontend::radeco_source::Source;

use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    {
        let mut r2 = R2::new(Some("/bin/ls")).expect("Failed to load r2");
        r2.analyze();
        let src: Rc<Source> = Rc::new(Rc::new(RefCell::new(r2)));
        let p = ProjectLoader::default()
            .path("/bin/ls")
            .source(Rc::clone(&src))
            .module_loader(
                ModuleLoader::default()
                    .parallel()
                    .build_ssa()
                    .build_callgraph()
                    .load_datarefs()
                    .function_loader(FunctionLoader::default().include_defaults()),
            )
            .load();

        for m in p.iter() {
            for rfn in m.module.iter() {
                println!("{:#X}", rfn.function.0);
            }
        }
    }
}
