// Copyright (c) 2015, The Radare Project. All rights reserved.
// See the COPYING file at the top-level directory of this distribution.
// Licensed under the BSD 3-Clause License:
// <http://opensource.org/licenses/BSD-3-Clause>
// This file may not be copied, modified, or distributed
// except according to those terms.

//! # The Radare2 Decompilation Library
//! radeco-lib is the crate that powers the
//! [radare2 decompiler](https://github.com/radare/radeco).
//!
//! Apart from decompilation, this library is designed to allow users to
//! perform static analysis on binaries in a easy and intuitive way.
//! __Reusablility__ and __Interactivity__ are the most important
//! design principles of this library.
//!
//! # Design
//! radeco-lib is built on top of r2pipe.rs, a simple library that provides
//! methods
//! to communicate with radare2 (using pipes). To know more about r2pipe or
//! radare2 in general, please head over to the
//! [repo](https://github.com/radare/radare2).
//!
//! radeco-lib works on analyzing ESIL (Evaluable Strings Intermediate
//! Language), an intermediate
//! representation (IR) used by radare2 for emulation. ESIL is converted into
//! an internal SSA IR
//! and used for subsequent analysis and optimizations.
//!
//! __NOTE__: This library is still under heavy developement.
//! Some API's have been stabilized, please check the docs before using
//! radeco-lib
//! in your projects as changes may not be backwards compatible. Contributions,
//! suggestions
//! and bug reports are always welcome at:
//! [tracker](https://github.com/radare/radeco-lib/issues)
//!

#![doc(html_root_url = "https://radare.github.io/radeco-lib/")]
#![doc(html_logo_url = "http://rada.re/r/img/r2logo3.png")]
#![feature(box_patterns)]
#![feature(box_syntax)]
#![feature(slice_patterns)]
#[cfg(test)] #[macro_use] extern crate quickcheck_macros;

extern crate petgraph;
extern crate regex;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;
extern crate bit_set;
extern crate either;
extern crate fixedbitset;
extern crate linear_map;
extern crate num;
extern crate typed_arena;
extern crate vec_map;

#[cfg(test)]
extern crate quickcheck;

#[cfg(feature = "trace_log")]
#[macro_use]
extern crate log;
#[cfg(feature = "trace_log")]
extern crate env_logger;

extern crate r2papi;
extern crate r2pipe;

extern crate esil;
// extern crate capstone_rust;
extern crate rayon;

#[cfg(feature = "profile")]
extern crate cpuprofiler;

extern crate lalrpop_util;

#[macro_use]
pub mod utils;
#[macro_use]
pub mod middle;
#[macro_use]
pub mod analysis;

pub mod backend;
pub mod frontend;
