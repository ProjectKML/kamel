#![allow(clippy::missing_safety_doc)]

pub mod backend;
pub mod renderer;

use std::ops::{Deref, DerefMut};

use kamel_bevy::{
    app::{self as bevy_app, App, AppLabel, Plugin},
    ecs::{self as bevy_ecs, schedule::StageLabel, world::World}
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    Render,
    Cleanup
}

#[derive(Default)]
pub struct RenderWorld(World);

impl Deref for RenderWorld {
    type Target = World;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RenderWorld {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

#[derive(Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = App::new();
        app.add_sub_app(RenderApp, render_app, |_app_world, _render_app| {});
    }
}
