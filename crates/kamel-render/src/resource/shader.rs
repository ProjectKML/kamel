use std::borrow::Cow;

use anyhow::{Error, Result};
use kamel_bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    reflect::{self as bevy_reflect, TypeUuid}
};

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "d09ec4a9-f995-429d-8924-d3cf6ddbc1bc"]
pub struct Shader {
    source: Source
}

impl Shader {
    pub fn from_hlsl(source: impl Into<Cow<'static, str>>) -> Self {
        Self {
            source: Source::Hlsl(source.into())
        }
    }

    pub fn from_spirv(source: impl Into<Cow<'static, [u8]>>) -> Self {
        Self {
            source: Source::SpirV(source.into())
        }
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    Hlsl(Cow<'static, str>),
    SpirV(Cow<'static, [u8]>)
}

#[derive(Default)]
pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let shader = match ext {
                "hlsl" => Shader::from_hlsl(String::from_utf8(Vec::from(bytes))?),
                "spv" => Shader::from_spirv(Vec::from(bytes)),
                _ => panic!("Unhandled extension: {}", ext)
            };

            let asset = LoadedAsset::new(shader);
            load_context.set_default_asset(asset);

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["hlsl", "spv"]
    }
}
