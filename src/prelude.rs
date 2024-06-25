//! This module re-exports a bunch of utilities used across this program.

#![allow(unused_imports)]

pub use mlua::prelude::*;
pub use tap::prelude::*;

pub use indexmap::{IndexMap, IndexSet};
pub use itertools::Itertools;
pub use thiserror::Error;

pub use log::debug;
pub use log::error;
pub use log::info;
pub use log::warn;

pub use crate::error::AppError;
pub use crate::error::AppResult;
pub use crate::util::text::FancyText;
pub use crate::util::notice::Notice;
pub use crate::util::notice::NoticePreset;

pub use crate::spec::{GameSpec, ModSpec};
