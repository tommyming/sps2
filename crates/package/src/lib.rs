#![deny(clippy::pedantic)]
#![allow(unsafe_code)] // Required for Starlark trait implementations
#![allow(clippy::module_name_repetitions)]
#![allow(unknown_lints)] // For CI compatibility across clippy versions
#![allow(clippy::elidable_lifetime_names)] // False positive from Starlark ProvidesStaticType derive macro

//! Starlark recipe handling for sps2
//!
//! This crate provides the sandboxed Starlark environment for build recipes,
//! exposing a limited API for package metadata and build operations.

mod error_helpers;
mod recipe;
mod sandbox;
mod starlark;

pub use recipe::{BuildStep, Recipe, RecipeMetadata};
pub use sandbox::{RecipeEngine, RecipeResult};
pub use starlark::{BuildContext, BuildExecutor};

use sps2_errors::Error;
use std::path::Path;

/// Load and parse a recipe file
///
/// # Errors
///
/// Returns a `BuildError::RecipeError` if the file cannot be read from the filesystem
/// or if the recipe content is invalid (missing required functions).
pub async fn load_recipe(path: &Path) -> Result<Recipe, Error> {
    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        sps2_errors::BuildError::RecipeError {
            message: format!("failed to read recipe: {e}"),
        }
    })?;

    Recipe::parse(&content)
}

/// Execute a recipe and get metadata
///
/// # Errors
///
/// Returns an error if the recipe execution fails, including:
/// - Starlark parsing or evaluation errors
/// - Missing or invalid metadata in the recipe
/// - Runtime errors during recipe execution
pub fn execute_recipe(recipe: &Recipe) -> Result<RecipeResult, Error> {
    let engine = RecipeEngine::new();
    engine.execute(recipe)
}

/// Extract metadata from a recipe without executing build steps
///
/// # Errors
///
/// Returns an error if:
/// - The recipe fails to parse as valid Starlark
/// - The metadata function fails or returns invalid data
/// - Required metadata fields are missing or invalid
pub fn extract_recipe_metadata(recipe: &Recipe) -> Result<RecipeMetadata, Error> {
    let engine = RecipeEngine::new();
    engine.extract_metadata(recipe)
}
