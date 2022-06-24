use std::sync::Arc;

use anyhow::bail;
use ash::{
    extensions::{khr, khr::GetSurfaceCapabilities2, nv::MeshShader},
    vk
};
use raw_window_handle::HasRawWindowHandle;

use crate::backend::{Device, Instance, Surface, Swapchain};

pub fn initialize(window: &impl HasRawWindowHandle) -> (Arc<Instance>, Arc<Surface>, Arc<Device>, Arc<Swapchain>) {
    let instance = Instance::new(window, |entry_loader, layers, extensions| unsafe {
        let version = entry_loader.try_enumerate_instance_version()?.unwrap_or(vk::API_VERSION_1_0);
        let major = vk::api_version_major(version);
        let minor = vk::api_version_minor(version);

        if major < 1 || minor < 1 {
            bail!(
                "Only Vulkan {}.{}.{} is supported, but minimum supported version is 1.1",
                major,
                minor,
                vk::api_version_patch(version)
            );
        }

        layers.push("VK_LAYER_KHRONOS_validation\0".as_ptr().cast());

        extensions.push(GetSurfaceCapabilities2::name().as_ptr());

        Ok(version)
    })
    .unwrap();

    let surface = Surface::new(instance.clone(), window).unwrap();

    let device = unsafe {
        Device::new(
            instance.clone(),
            surface.clone(),
            instance.find_optimal_physical_device(),
            |properties, _memory_properties, _queue_family_properties, extensions, _supported_features, _enabled_features| {
                let version = properties.properties.api_version;
                let major = vk::api_version_minor(version);
                let minor = vk::api_version_minor(version);

                if major < 1 || minor < 1 {
                    bail!(
                        "Only Vulkan {}.{}.{} is supported, but minimum supported version is 1.1",
                        major,
                        minor,
                        vk::api_version_patch(version)
                    );
                }

                extensions.try_push(b"VK_KHR_portability_subset\0".as_ptr().cast());
                extensions.push(khr::Swapchain::name().as_ptr());
                extensions.try_push(MeshShader::name().as_ptr());

                Ok(())
            }
        )
        .unwrap()
    };

    let swapchain = Swapchain::new(instance.clone(), surface.clone(), device.clone(), true).unwrap();

    (instance, surface, device, swapchain)
}
