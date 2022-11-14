#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), deny(unused_crate_dependencies))]
#![deny(missing_docs, unreachable_pub)]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]

mod event_formatter;
mod google;
mod layer;
mod serializers;
mod visitor;
mod writer;

pub use self::google::*;
pub use self::layer::*;
