// Copyright Ion Fusion contributors. All Rights Reserved.

use crate::config::FusionConfig;
use crate::error::Error;
use crate::file::{FusionFile, FusionFileContent};

mod module;
mod scope;

use module::load_module_into_scope;
pub use scope::*;
