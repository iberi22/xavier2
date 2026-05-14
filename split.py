import os

with open('scratch/system3_old_utf8.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

os.makedirs('src/agents/system3', exist_ok=True)

types_rs = '''use serde::{Deserialize, Serialize};

''' + ''.join(lines[1992:2066])
with open('src/agents/system3/types.rs', 'w', encoding='utf-8') as f: f.write(types_rs)

client_rs = '''use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::agents::provider::ModelProviderClient;
use crate::agents::system1::RetrievedDocument;

''' + ''.join(lines[20:93])
with open('src/agents/system3/client.rs', 'w', encoding='utf-8') as f: f.write(client_rs)

helpers_rs = '''use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use crate::agents::system1::RetrievedDocument;
use crate::utils::crypto::sha256_hex;

''' + ''.join(lines[94:1382])
with open('src/agents/system3/helpers.rs', 'w', encoding='utf-8') as f: f.write(helpers_rs)

engine_rs = '''use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};
use crate::agents::system1::{RetrievalResult, RetrievedDocument};
use crate::agents::system2::ReasoningResult;
use super::types::{ActionResult, ActorConfig};
use super::client::LlmClient;
use super::helpers::*;

''' + ''.join(lines[1383:1491]) + ''.join(lines[2067:2223])
with open('src/agents/system3/engine.rs', 'w', encoding='utf-8') as f: f.write(engine_rs)

tests_rs = ''.join(lines[1492:1991])
with open('src/agents/system3/tests.rs', 'w', encoding='utf-8') as f: f.write(tests_rs)

mod_rs = '''pub mod types;
pub mod client;
pub mod helpers;
pub mod engine;
#[cfg(test)]
pub mod tests;

pub use engine::System3Actor;
pub use types::{ActionResult, ActorConfig, Action, ActionType, MemoryUpdate, MemoryOperation, ToolCall};
'''
with open('src/agents/system3/mod.rs', 'w', encoding='utf-8') as f: f.write(mod_rs)
