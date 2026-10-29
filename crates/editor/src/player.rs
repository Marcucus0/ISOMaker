// Build the same window/render loop as the editor under a player-specific
// binary name. The shared entry point selects the project-only runtime mode.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
include!("main.rs");
