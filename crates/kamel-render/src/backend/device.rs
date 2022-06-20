use std::{os::raw::c_char, sync::Arc};

use anyhow::Result;
use ash::{
    extensions::{khr::Swapchain, nv::MeshShader},
    prelude::VkResult,
    vk
};

use crate::backend::{Instance, Surface};

pub struct Properties {
    pub properties: vk::PhysicalDeviceProperties,
    pub mesh_shader_properties: vk::PhysicalDeviceMeshShaderPropertiesNV<'static>
}

impl Properties {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut mesh_shader_properties = vk::PhysicalDeviceMeshShaderPropertiesNV::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut mesh_shader_properties);

        instance.loader().get_physical_device_properties2(physical_device, &mut properties);

        Self {
            properties: properties.properties,
            mesh_shader_properties
        }
    }
}

unsafe impl Send for Properties {}
unsafe impl Sync for Properties {}

pub struct MemoryProperties {
    pub memory_properties: vk::PhysicalDeviceMemoryProperties
}

impl MemoryProperties {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::default();

        instance.loader().get_physical_device_memory_properties2(physical_device, &mut memory_properties);

        Self {
            memory_properties: memory_properties.memory_properties
        }
    }
}

pub struct QueueFamilyProperties {
    pub queue_family_properties: Vec<vk::QueueFamilyProperties>
}

impl QueueFamilyProperties {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let instance_loader = instance.loader();

        let mut queue_family_properties: Vec<_> = (0..instance_loader.get_physical_device_queue_family_properties2_len(physical_device))
            .into_iter()
            .map(|_| vk::QueueFamilyProperties2::default())
            .collect();
        instance_loader.get_physical_device_queue_family_properties2(physical_device, &mut queue_family_properties);

        Self {
            queue_family_properties: queue_family_properties
                .iter()
                .map(|queue_family_properties| queue_family_properties.queue_family_properties)
                .collect()
        }
    }
}

#[derive(Default)]
pub struct Features {
    pub features: vk::PhysicalDeviceFeatures,
    pub mesh_shader_features: vk::PhysicalDeviceMeshShaderFeaturesNV<'static>
}

impl Features {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut mesh_shader_features = vk::PhysicalDeviceMeshShaderFeaturesNV::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut mesh_shader_features);

        instance.loader().get_physical_device_features2(physical_device, &mut features);

        Self {
            features: features.features,
            mesh_shader_features
        }
    }
}

unsafe impl Send for Features {}
unsafe impl Sync for Features {}

pub struct Extensions {
    supported: Vec<vk::ExtensionProperties>,
    enabled: Vec<*const c_char>,

    khr_portability_subset: bool,
    khr_swapchain: bool,
    nv_mesh_shader: bool
}

impl Extensions {
    #[inline]
    pub unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> VkResult<Self> {
        let supported = instance.loader().enumerate_device_extension_properties(physical_device)?;

        Ok(Self {
            supported,
            enabled: Vec::new(),
            khr_portability_subset: false,
            khr_swapchain: false,
            nv_mesh_shader: false
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

        if libc::strcmp(name, b"VK_KHR_portability_subset\0".as_ptr().cast()) == 0 {
            self.khr_portability_subset = true;
        } else if libc::strcmp(name, Swapchain::name().as_ptr()) == 0 {
            self.khr_swapchain = true;
        } else if libc::strcmp(name, MeshShader::name().as_ptr()) == 0 {
            self.nv_mesh_shader = true;
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
    pub fn khr_swapchain(&self) -> bool {
        self.khr_swapchain
    }

    #[inline]
    pub fn nv_mesh_shader(&self) -> bool {
        self.nv_mesh_shader
    }
}

pub struct Queue {
    queue: vk::Queue,
    family_index: u32
}

impl Queue {
    unsafe fn new(device_loader: &ash::Device, family_index: u32) -> Self {
        Self {
            queue: device_loader.get_device_queue(family_index, 0),
            family_index
        }
    }

    #[inline]
    pub fn queue(&self) -> &vk::Queue {
        &self.queue
    }

    #[inline]
    pub fn family_index(&self) -> u32 {
        self.family_index
    }
}

pub struct Device {
    physical_device: vk::PhysicalDevice,

    loader: Arc<ash::Device>,
    swapchain_loader: Swapchain,
    mesh_shader_loader: MeshShader,

    extensions: Extensions,

    properties: Properties,
    memory_properties: MemoryProperties,
    queue_family_properties: QueueFamilyProperties,

    supported_features: Features,
    enabled_features: Features,

    direct_queue: Queue,

    compute_queue: Queue,

    transfer_queue: Queue,

    _instance: Arc<Instance>,
    _surface: Arc<Surface>
}

unsafe fn find_direct_queue_family_index(instance: &Instance, surface: &Surface, physical_device: vk::PhysicalDevice, properties: &[vk::QueueFamilyProperties]) -> Option<u32> {
    let mut queue_count: u32 = 0;
    let mut family_index: u32 = 0;

    let direct_flags: vk::QueueFlags = vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER;

    for (i, properties) in properties.iter().enumerate() {
        let i = i as u32;

        if (properties.queue_flags & direct_flags) == direct_flags
            && properties.queue_count > queue_count
            && instance
                .surface_loader()
                .get_physical_device_surface_support(physical_device, i, *surface.surface())
                .unwrap_or(false)
        {
            queue_count = properties.queue_count;
            family_index = i;
        }
    }

    if queue_count > 0 {
        Some(family_index)
    } else {
        None
    }
}

unsafe fn find_queue_family_index(properties: &[vk::QueueFamilyProperties], desired_flags: vk::QueueFlags, undesired_flags: vk::QueueFlags) -> Option<u32> {
    let mut queue_count: u32 = 0;
    let mut family_index: u32 = 0;

    for (i, properties) in properties.iter().enumerate() {
        let i = i as u32;

        if (properties.queue_flags & desired_flags) == desired_flags && (properties.queue_flags & undesired_flags) == vk::QueueFlags::empty() && properties.queue_count > queue_count {
            queue_count = properties.queue_count;
            family_index = i;
        }
    }

    if queue_count > 0 {
        Some(family_index)
    } else {
        None
    }
}

unsafe fn find_queue_family_indices(instance: &Instance, surface: &Surface, physical_device: vk::PhysicalDevice, properties: &[vk::QueueFamilyProperties]) -> Option<(u32, u32, u32)> {
    let direct_index = find_direct_queue_family_index(instance, surface, physical_device, properties)?;
    let compute_index = find_queue_family_index(properties, vk::QueueFlags::COMPUTE, vk::QueueFlags::GRAPHICS | vk::QueueFlags::TRANSFER)
        .or_else(|| find_queue_family_index(properties, vk::QueueFlags::COMPUTE, vk::QueueFlags::GRAPHICS))
        .or_else(|| find_queue_family_index(properties, vk::QueueFlags::COMPUTE, vk::QueueFlags::TRANSFER))
        .unwrap_or(direct_index);

    let transfer_index = find_queue_family_index(properties, vk::QueueFlags::TRANSFER, vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE)
        .or_else(|| find_queue_family_index(properties, vk::QueueFlags::TRANSFER, vk::QueueFlags::GRAPHICS))
        .or_else(|| find_queue_family_index(properties, vk::QueueFlags::TRANSFER, vk::QueueFlags::COMPUTE))
        .unwrap_or(direct_index);

    Some((direct_index, compute_index, transfer_index))
}

impl Device {
    pub unsafe fn new(
        instance: Arc<Instance>,
        surface: Arc<Surface>,
        physical_device: vk::PhysicalDevice,
        callback: impl FnOnce(&Properties, &MemoryProperties, &QueueFamilyProperties, &mut Extensions, &Features, &mut Features) -> Result<()>
    ) -> Result<Arc<Self>> {
        let mut extensions = Extensions::new(&instance, physical_device)?;

        let properties = Properties::new(&instance, physical_device);
        let memory_properties = MemoryProperties::new(&instance, physical_device);
        let queue_family_properties = QueueFamilyProperties::new(&instance, physical_device);

        let supported_features = Features::new(&instance, physical_device);
        let mut enabled_features = Features::default();

        callback(
            &properties,
            &memory_properties,
            &queue_family_properties,
            &mut extensions,
            &supported_features,
            &mut enabled_features
        )?;

        //Queue families
        let (direct_queue_family_index, compute_queue_family_index, transfer_queue_family_index) =
            find_queue_family_indices(&instance, &surface, physical_device, &queue_family_properties.queue_family_properties)
                .ok_or_else(|| anyhow::anyhow!("Failed to find queue family indices"))?;

        let queue_priorities = [1.0];

        let mut device_queue_create_infos = vec![vk::DeviceQueueCreateInfo::default()
            .queue_family_index(direct_queue_family_index)
            .queue_priorities(&queue_priorities)];

        if compute_queue_family_index != direct_queue_family_index {
            device_queue_create_infos.push(
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(compute_queue_family_index)
                    .queue_priorities(&queue_priorities)
            );
        }

        if transfer_queue_family_index != direct_queue_family_index {
            device_queue_create_infos.push(
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(transfer_queue_family_index)
                    .queue_priorities(&queue_priorities)
            );
        }

        //Features
        let mut mesh_shader_features = enabled_features.mesh_shader_features;
        let mut features = vk::PhysicalDeviceFeatures2::default().features(enabled_features.features).push_next(&mut mesh_shader_features);

        //Create device
        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&device_queue_create_infos)
            .enabled_extension_names(extensions.enabled())
            .push_next(&mut features);

        let instance_loader = instance.loader();
        let loader = Arc::new(instance_loader.create_device(physical_device, &device_create_info, None)?);
        let swapchain_loader = Swapchain::new(instance_loader, &loader);
        let mesh_shader_loader = MeshShader::new(instance_loader, &loader);

        //TODO: allocator

        let direct_queue = Queue::new(&loader, direct_queue_family_index);
        let compute_queue = Queue::new(&loader, compute_queue_family_index);
        let transfer_queue = Queue::new(&loader, transfer_queue_family_index);

        Ok(Arc::new(Self {
            physical_device,

            loader,
            swapchain_loader,
            mesh_shader_loader,

            //TODO: allocator,
            extensions,

            properties,
            memory_properties,
            queue_family_properties,

            supported_features,
            enabled_features,

            direct_queue,
            compute_queue,
            transfer_queue,

            _instance: instance,
            _surface: surface
        }))
    }

    #[inline]
    pub fn physical_device(&self) -> &vk::PhysicalDevice {
        &self.physical_device
    }

    #[inline]
    pub fn loader(&self) -> &Arc<ash::Device> {
        &self.loader
    }

    #[inline]
    pub fn swapchain_loader(&self) -> &Swapchain {
        &self.swapchain_loader
    }

    #[inline]
    pub fn mesh_shader_loader(&self) -> &MeshShader {
        &self.mesh_shader_loader
    }

    //TODO: allocator getter

    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    #[inline]
    pub fn properties(&self) -> &Properties {
        &self.properties
    }

    #[inline]
    pub fn memory_properties(&self) -> &MemoryProperties {
        &self.memory_properties
    }

    #[inline]
    pub fn queue_family_properties(&self) -> &QueueFamilyProperties {
        &self.queue_family_properties
    }

    #[inline]
    pub fn supported_features(&self) -> &Features {
        &self.supported_features
    }

    #[inline]
    pub fn enabled_features(&self) -> &Features {
        &self.enabled_features
    }

    #[inline]
    pub fn direct_queue(&self) -> &Queue {
        &self.direct_queue
    }

    #[inline]
    pub fn compute_queue(&self) -> &Queue {
        &self.compute_queue
    }

    #[inline]
    pub fn transfer_queue(&self) -> &Queue {
        &self.transfer_queue
    }
}

impl Drop for Device {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_device(None);
        }
    }
}
