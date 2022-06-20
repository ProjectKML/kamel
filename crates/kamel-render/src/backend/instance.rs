use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
    sync::Arc
};

use anyhow::Result;
use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{GetSurfaceCapabilities2, Surface}
    },
    prelude::VkResult,
    vk, Entry
};
use log::log;
use raw_window_handle::HasRawWindowHandle;

use crate::backend::util::message_severity;

#[inline]
fn application_info_from_cargo_toml(api_version: u32) -> vk::ApplicationInfo<'static> {
    let version = vk::make_api_version(
        0,
        env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_PATCH").parse().unwrap()
    );

    let application_name = concat!(env!("CARGO_PKG_NAME"), "_game\0");
    let engine_name = concat!(env!("CARGO_PKG_NAME"), "\0");

    unsafe {
        vk::ApplicationInfo::default()
            .application_name(CStr::from_bytes_with_nul_unchecked(application_name.as_bytes()))
            .application_version(version)
            .engine_name(CStr::from_bytes_with_nul_unchecked(engine_name.as_bytes()))
            .engine_version(version)
            .api_version(api_version)
    }
}

pub struct Layers {
    supported: Vec<vk::LayerProperties>,
    enabled: Vec<*const c_char>,

    khronos_validation: bool
}

impl Layers {
    pub fn new(entry_loader: &Entry) -> VkResult<Self> {
        let supported = entry_loader.enumerate_instance_layer_properties()?;

        Ok(Self {
            supported,
            enabled: Vec::new(),

            khronos_validation: false
        })
    }

    #[inline]
    pub unsafe fn is_supported(&self, name: *const c_char) -> bool {
        self.supported.iter().any(|e| libc::strcmp(e.layer_name.as_ptr(), name) == 0)
    }

    #[inline]
    pub unsafe fn is_enabled(&self, name: *const c_char) -> bool {
        self.enabled.iter().any(|e| libc::strcmp(*e, name) == 0)
    }

    #[inline]
    pub unsafe fn try_push(&mut self, name: *const c_char) -> bool {
        if !self.is_supported(name) || self.is_enabled(name) {
            return false
        }

        self.enabled.push(name);

        if libc::strcmp(name, b"VK_LAYER_KHRONOS_validation\0".as_ptr().cast()) == 0 {
            self.khronos_validation = true;
        }

        true
    }

    #[inline]
    pub unsafe fn push(&mut self, name: *const c_char) {
        assert!(self.try_push(name))
    }

    #[inline]
    pub fn supported(&self) -> &Vec<vk::LayerProperties> {
        &self.supported
    }

    #[inline]
    pub fn enabled(&self) -> &Vec<*const c_char> {
        &self.enabled
    }

    #[inline]
    pub fn khronos_validation(&self) -> bool {
        self.khronos_validation
    }
}

pub struct Extensions {
    supported: Vec<vk::ExtensionProperties>,
    enabled: Vec<*const c_char>,

    ext_debug_utils: bool,
    khr_get_surface_capabilities2: bool,
    khr_surface: bool
}

impl Extensions {
    #[inline]
    pub fn new(entry_loader: &Entry) -> VkResult<Self> {
        let supported = entry_loader.enumerate_instance_extension_properties(None)?;

        Ok(Self {
            supported,
            enabled: Vec::new(),

            ext_debug_utils: false,
            khr_get_surface_capabilities2: false,
            khr_surface: false
        })
    }

    #[inline]
    pub unsafe fn is_supported(&self, name: *const c_char) -> bool {
        self.supported.iter().any(|e| libc::strcmp(e.extension_name.as_ptr(), name) == 0)
    }

    #[inline]
    pub unsafe fn is_enabled(&self, name: *const c_char) -> bool {
        self.enabled.iter().any(|e| libc::strcmp(*e, name) == 0)
    }

    #[inline]
    pub unsafe fn try_push(&mut self, name: *const c_char) -> bool {
        if !self.is_supported(name) || self.is_enabled(name) {
            return false
        }

        self.enabled.push(name);

        if libc::strcmp(name, DebugUtils::name().as_ptr()) == 0 {
            self.ext_debug_utils = true;
        } else if libc::strcmp(name, GetSurfaceCapabilities2::name().as_ptr()) == 0 {
            self.khr_get_surface_capabilities2 = true;
        } else if libc::strcmp(name, Surface::name().as_ptr()) == 0 {
            self.khr_surface = true;
        }

        true
    }

    #[inline]
    pub unsafe fn push(&mut self, name: *const c_char) {
        assert!(self.try_push(name))
    }

    #[inline]
    pub fn supported(&self) -> &Vec<vk::ExtensionProperties> {
        &self.supported
    }

    #[inline]
    pub fn enabled(&self) -> &Vec<*const c_char> {
        &self.enabled
    }

    #[inline]
    pub fn ext_debug_utils(&self) -> bool {
        self.ext_debug_utils
    }

    #[inline]
    pub fn khr_get_surface_capabilities2(&self) -> bool {
        self.khr_get_surface_capabilities2
    }

    #[inline]
    pub fn khr_surface(&self) -> bool {
        self.khr_surface
    }
}

pub struct Instance {
    entry_loader: Entry,

    loader: Arc<ash::Instance>,
    debug_utils_loader: DebugUtils,
    get_surface_capabilities2_loader: GetSurfaceCapabilities2,
    surface_loader: Surface,

    layers: Layers,
    extensions: Extensions,

    debug_utils_messenger: vk::DebugUtilsMessengerEXT,

    physical_devices: Vec<vk::PhysicalDevice>
}

impl Instance {
    pub fn new(window: &impl HasRawWindowHandle, callback: impl FnOnce(&Entry, &mut Layers, &mut Extensions) -> Result<u32>) -> Result<Arc<Self>> {
        unsafe {
            let entry_loader = Entry::load()?;

            //Layers
            let mut layers = Layers::new(&entry_loader)?;
            let mut extensions = Extensions::new(&entry_loader)?;
            ash_window::enumerate_required_extensions(&window)?.iter().for_each(|e| extensions.push(*e));

            let application_info = application_info_from_cargo_toml(callback(&entry_loader, &mut layers, &mut extensions)?);

            let instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(&application_info)
                .enabled_extension_names(extensions.enabled())
                .enabled_layer_names(layers.enabled());

            let loader = Arc::new(entry_loader.create_instance(&instance_create_info, None)?);
            let debug_utils_loader = DebugUtils::new(&entry_loader, &loader);
            let get_surface_capabilities2_loader = GetSurfaceCapabilities2::new(&entry_loader, &loader);
            let surface_loader = Surface::new(&entry_loader, &loader);

            let debug_utils_messenger = if extensions.ext_debug_utils() {
                let debug_utils_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    )
                    .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
                    .pfn_user_callback(Some(debug_callback));

                debug_utils_loader.create_debug_utils_messenger(&debug_utils_messenger_create_info, None)?
            } else {
                vk::DebugUtilsMessengerEXT::null()
            };

            let physical_devices = loader.enumerate_physical_devices()?;

            Ok(Arc::new(Self {
                entry_loader,

                loader,
                debug_utils_loader,
                get_surface_capabilities2_loader,
                surface_loader,

                layers,
                extensions,

                debug_utils_messenger,

                physical_devices
            }))
        }
    }

    pub fn find_optimal_physical_device(&self) -> vk::PhysicalDevice {
        let mut heap_size: u64 = 0;
        let mut physical_device = vk::PhysicalDevice::null();

        for current_physical_device in self.physical_devices.iter() {
            let properties = unsafe { self.loader.get_physical_device_properties(*current_physical_device) };

            if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
                continue
            }

            let memory_properties = unsafe { self.loader.get_physical_device_memory_properties(*current_physical_device) };
            let mut current_heap_size: u64 = 0;

            for i in 0..memory_properties.memory_heap_count as usize {
                let current_heap = &memory_properties.memory_heaps[i];

                if (current_heap.flags & vk::MemoryHeapFlags::DEVICE_LOCAL) == vk::MemoryHeapFlags::DEVICE_LOCAL {
                    current_heap_size += current_heap.size;
                }
            }

            if current_heap_size > heap_size {
                heap_size = current_heap_size;
                physical_device = *current_physical_device;
            }
        }

        if physical_device == vk::PhysicalDevice::null() {
            physical_device = self.physical_devices[0];
        }

        physical_device
    }

    #[inline]
    pub fn entry_loader(&self) -> &Entry {
        &self.entry_loader
    }

    #[inline]
    pub fn loader(&self) -> &Arc<ash::Instance> {
        &self.loader
    }

    #[inline]
    pub fn debug_utils_loader(&self) -> &DebugUtils {
        &self.debug_utils_loader
    }

    #[inline]
    pub fn get_surface_capabilities2_loader(&self) -> &GetSurfaceCapabilities2 {
        &self.get_surface_capabilities2_loader
    }

    #[inline]
    pub fn surface_loader(&self) -> &Surface {
        &self.surface_loader
    }

    #[inline]
    pub fn layers(&self) -> &Layers {
        &self.layers
    }

    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }
}

impl Drop for Instance {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.debug_utils_messenger != vk::DebugUtilsMessengerEXT::null() {
                self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            }

            self.loader.destroy_instance(None);
        }
    }
}

unsafe impl Send for Instance {}
unsafe impl Sync for Instance {}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void
) -> vk::Bool32 {
    log!(
        message_severity::to_log_level(message_severity),
        "[{:?}]{}",
        message_types,
        CStr::from_ptr((*callback_data).p_message).to_str().unwrap()
    );

    vk::FALSE
}
