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

pub use crate::structs::error::{AppError, AppResult, ModpackError, GameError};
pub use crate::util::misc::SaveLoad;
pub use crate::util::text::FancyText;
pub use crate::util::notice::Notice;
pub use crate::util::notice::NoticePreset;

pub use crate::structs::data::AppData;
pub use crate::structs::config::{AppConfig, TargetGame};
pub use crate::structs::spec::{GameSpec, ModSpec};
