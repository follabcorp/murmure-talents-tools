//! Adapters: the command line and the disk.

mod cli;
mod fs_catalog_disk;

pub use cli::{run, CliResult};
pub use fs_catalog_disk::FsCatalogDisk;
