use std::{slice, sync::Arc};

use anyhow::Result;
use ash::{prelude::VkResult, vk};

use crate::backend::{Device, Instance, Surface};

pub struct SurfaceCapabilities {
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR
}

impl SurfaceCapabilities {
    #[inline]
    pub unsafe fn new(instance: &Instance, device: &Device, surface_info: &vk::PhysicalDeviceSurfaceInfo2KHR) -> VkResult<Self> {
        Ok(Self {
            surface_capabilities: instance
                .get_surface_capabilities2_loader()
                .get_physical_device_surface_capabilities2(*device.physical_device(), surface_info)?.surface_capabilities
        })
    }
}

pub struct SurfaceFormats {
    pub supported_formats: Vec<vk::SurfaceFormatKHR>
}

impl SurfaceFormats {
    #[inline]
    pub unsafe fn new(instance: &Instance, device: &Device, surface_info: &vk::PhysicalDeviceSurfaceInfo2KHR) -> VkResult<Self> {
        let get_surface_capabilities2_loader = instance.get_surface_capabilities2_loader();
        let physical_device = *device.physical_device();

        let mut supported_formats: Vec<_> = (0..get_surface_capabilities2_loader.get_physical_device_surface_formats2_len(physical_device, surface_info)?)
            .into_iter()
            .map(|_| vk::SurfaceFormat2KHR::default())
            .collect();
        get_surface_capabilities2_loader.get_physical_device_surface_formats2(physical_device, surface_info, &mut supported_formats)?;

        Ok(Self { supported_formats: supported_formats.iter().map(|format| format.surface_format).collect() })
    }

    #[inline]
    pub fn find_ldr_format(&self) -> Option<vk::SurfaceFormatKHR> {
        const FORMATS: [vk::Format; 4] = [vk::Format::R8G8B8A8_SRGB, vk::Format::B8G8R8A8_SRGB, vk::Format::R8G8B8A8_UNORM, vk::Format::B8G8R8A8_UNORM];

        self.supported_formats.iter().find(|f| FORMATS.contains(&f.format)).map(|f| *f)
    }

    #[inline]
    pub fn find_hdr_format(&self) -> Option<vk::SurfaceFormatKHR> {
        self.supported_formats
            .iter()
            .find(|f| f.format == vk::Format::R16G16B16A16_SFLOAT && f.color_space == vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT)
            .map(|f| *f)
    }
}

pub struct Swapchain {
    surface_capabilities: SurfaceCapabilities,

    surface_formats: SurfaceFormats,
    present_modes: Vec<vk::PresentModeKHR>,

    render_pass: vk::RenderPass,
    _images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,

    used_surface_format: vk::SurfaceFormatKHR,
    used_present_mode: vk::PresentModeKHR,
    vsync_enabled: bool,

    swapchain: vk::SwapchainKHR,

    _instance: Arc<Instance>,
    _surface: Arc<Surface>,
    device: Arc<Device>
}

impl Swapchain {
    unsafe fn create_render_pass(device: &Device, format: vk::Format) -> VkResult<vk::RenderPass> {
        let attachment_description = vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_reference = vk::AttachmentReference::default().layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass_description = vk::SubpassDescription::default().color_attachments(slice::from_ref(&color_attachment_reference));

        let render_pass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(slice::from_ref(&attachment_description))
            .subpasses(slice::from_ref(&subpass_description));

        device.loader().create_render_pass(&render_pass_create_info, None)
    }

    #[allow(clippy::type_complexity)]
    unsafe fn create_swapchain(
        device: &Device,
        surface: vk::SurfaceKHR,
        render_pass: vk::RenderPass,
        surface_capabilities: &SurfaceCapabilities,
        used_surface_format: &vk::SurfaceFormatKHR,
        used_present_mode: vk::PresentModeKHR,
        old_swapchain: vk::SwapchainKHR
    ) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>, Vec<vk::Framebuffer>)> {
        let device_loader = device.loader();
        let surface_capabilities = &surface_capabilities.surface_capabilities;

        let min_image_count = 3.max(surface_capabilities.min_image_count);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(min_image_count)
            .image_format(used_surface_format.format)
            .image_color_space(used_surface_format.color_space)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(used_present_mode)
            .old_swapchain(old_swapchain);

        let swapchain_loader = device.swapchain_loader();
        let swapchain = swapchain_loader.create_swapchain(&swapchain_create_info, None)?;

        let images = swapchain_loader.get_swapchain_images(swapchain)?;
        let mut image_views = Vec::with_capacity(images.len());
        let mut framebuffers = Vec::with_capacity(images.len());

        let mut image_view_create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(swapchain_create_info.image_format)
            .subresource_range(vk::ImageSubresourceRange::default().aspect_mask(vk::ImageAspectFlags::COLOR).level_count(1).layer_count(1));

        let mut framebuffer_create_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .width(swapchain_create_info.image_extent.width)
            .height(swapchain_create_info.image_extent.height)
            .layers(1);
        framebuffer_create_info.attachment_count = 1;

        for image in images.iter() {
            image_view_create_info.image = *image;
            let image_view = device_loader.create_image_view(&image_view_create_info, None)?;
            image_views.push(image_view);

            framebuffer_create_info.p_attachments = &image_view;
            framebuffers.push(device_loader.create_framebuffer(&framebuffer_create_info, None)?);
        }

        Ok((swapchain, images, image_views, framebuffers))
    }

    pub fn new(instance: Arc<Instance>, surface: Arc<Surface>, device: Arc<Device>, vsync_enabled: bool) -> Result<Arc<Self>> {
        let surface_handle = *surface.surface();
        let surface_info = vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(surface_handle);

        unsafe {
            let surface_capabilities = SurfaceCapabilities::new(&instance, &device, &surface_info)?;

            let surface_formats = SurfaceFormats::new(&instance, &device, &surface_info)?;
            let present_modes = instance.surface_loader().get_physical_device_surface_present_modes(*device.physical_device(), surface_handle)?;
            let get_present_mode_if_supported = |present_mode: vk::PresentModeKHR| present_modes.iter().find(|p| **p == present_mode).copied();

            let used_surface_format = surface_formats
                .find_hdr_format()
                .or_else(|| surface_formats.find_ldr_format())
                .ok_or_else(|| anyhow::anyhow!("Failed to find surface format"))?;

            let used_present_mode = if vsync_enabled {
                vk::PresentModeKHR::FIFO
            } else {
                get_present_mode_if_supported(vk::PresentModeKHR::IMMEDIATE)
                    .or_else(|| get_present_mode_if_supported(vk::PresentModeKHR::MAILBOX))
                    .unwrap_or(vk::PresentModeKHR::FIFO)
            };

            let render_pass = Self::create_render_pass(&device, used_surface_format.format)?;
            let (swapchain, images, image_views, framebuffers) = Self::create_swapchain(
                &device,
                surface_handle,
                render_pass,
                &surface_capabilities,
                &used_surface_format,
                used_present_mode,
                vk::SwapchainKHR::null()
            )?;

            Ok(Arc::new(Self {
                surface_capabilities,

                surface_formats,
                present_modes,

                render_pass,
                _images: images,
                image_views,
                framebuffers,

                used_present_mode,
                used_surface_format,
                vsync_enabled,

                swapchain,

                _instance: instance,
                _surface: surface,
                device
            }))
        }
    }

    #[inline]
    pub fn surface_capabilities(&self) -> &SurfaceCapabilities {
        &self.surface_capabilities
    }

    #[inline]
    pub fn surface_formats(&self) -> &SurfaceFormats {
        &self.surface_formats
    }

    #[inline]
    pub fn present_modes(&self) -> &[vk::PresentModeKHR] {
        &self.present_modes
    }

    #[inline]
    pub fn render_pass(&self) -> &vk::RenderPass {
        &self.render_pass
    }

    #[inline]
    pub fn framebuffer_at(&self, index: usize) -> &vk::Framebuffer {
        &self.framebuffers[index]
    }

    #[inline]
    pub fn used_surface_format(&self) -> vk::SurfaceFormatKHR {
        self.used_surface_format
    }

    #[inline]
    pub fn used_present_mode(&self) -> vk::PresentModeKHR {
        self.used_present_mode
    }

    #[inline]
    pub fn vsync_enabled(&self) -> bool {
        self.vsync_enabled
    }

    #[inline]
    pub fn swapchain(&self) -> &vk::SwapchainKHR {
        &self.swapchain
    }
}

impl Drop for Swapchain {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let device_loader = self.device.loader();

            self.framebuffers.iter().for_each(|framebuffer| device_loader.destroy_framebuffer(*framebuffer, None));
            self.image_views.iter().for_each(|image_view| device_loader.destroy_image_view(*image_view, None));

            self.device.swapchain_loader().destroy_swapchain(self.swapchain, None);

            device_loader.destroy_render_pass(self.render_pass, None);
        }
    }
}

unsafe impl Send for Swapchain {}
unsafe impl Sync for Swapchain {}
