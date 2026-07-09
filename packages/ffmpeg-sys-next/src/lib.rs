#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::approx_constant)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
// Bindgen output is compiler-generated and intentionally follows the C ABI verbatim.
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::useless_transmute)]
#![allow(unnecessary_transmutes)]
#![allow(suspicious_runtime_symbol_definitions)]
#![allow(unpredictable_function_pointer_comparisons)]

extern crate ffmpeg_pipeline_sdk;
extern crate libc;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[macro_use]
mod avutil;
pub use avutil::*;
