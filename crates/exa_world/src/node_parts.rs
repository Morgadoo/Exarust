//! A drawable node's parts: the original's `TGeometry` keeps the list of
//! its sub-objects (one per mesh part, each with its material), and a few
//! of its rules address "the first sub-object" — a light's flicker and
//! lever scale the emissive of their parent mesh's first one
//! (`crate::light_source`). Spawning a model creates one part entity per
//! mesh under its node and records them here in the model's part order.

use bevy::prelude::*;

/// A model node's part entities, in the model's part order.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeParts {
    pub parts: Vec<Entity>,
}

impl NodeParts {
    pub fn new(parts: Vec<Entity>) -> Self {
        Self { parts }
    }
}
