#![warn(missing_crate_level_docs, missing_docs)]
//! Crate containing common functions and types used in Garage

#[macro_use]
extern crate log;

pub mod background;
pub mod config;
pub mod data;
pub mod error;
pub mod persister;
pub mod time;
