use std::sync::Arc;

use ash::{prelude::VkResult, vk};
use vk_mem::{Allocation, AllocationCreateInfo, AllocationInfo, MemoryUsage};

use crate::backend::Device;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct BufferDesc {
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub memory_usage: MemoryUsage
}

impl BufferDesc {
    #[inline]
    pub fn new_gpu_only(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            memory_usage: MemoryUsage::GpuOnly
        }
    }

    #[inline]
    pub fn new_cpu_only(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            memory_usage: MemoryUsage::CpuOnly
        }
    }

    #[inline]
    pub fn new_cpu_to_gpu(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            memory_usage: MemoryUsage::CpuToGpu
        }
    }

    #[inline]
    pub fn new_gpu_to_cpu(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            memory_usage: MemoryUsage::GpuToCpu
        }
    }

    #[inline]
    pub fn new_gpu_lazy(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            memory_usage: MemoryUsage::GpuLazy
        }
    }
}

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    allocation_info: AllocationInfo,
    device_address: vk::DeviceAddress,

    device: Arc<Device>
}

impl Buffer {
    pub fn new(device: Arc<Device>, desc: &BufferDesc) -> VkResult<Self> {
        let buffer_create_info = vk::BufferCreateInfo::default().size(desc.size).usage(buffer_desc.usage);

        let allocation_create_info = AllocationCreateInfo::new().usage(desc.memory_usage);

        let (buffer, allocation, allocation_info) = unsafe { device.allocator().create_buffer(&buffer_create_info, &allocation_create_info)? };

        let device_address = if (desc.usage & vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) == vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS {
            unsafe { device.loader().get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(buffer)) }
        } else {
            0
        };

        Ok(Self {
            buffer,
            allocation,
            allocation_info,
            device_address,
            device
        })
    }

    #[inline]
    pub fn buffer(&self) -> &vk::Buffer {
        &self.buffer
    }

    #[inline]
    pub fn allocation(&self) -> &Allocation {
        &self.allocation
    }

    #[inline]
    pub fn allocation_info(&self) -> &AllocationInfo {
        &self.allocation_info
    }

    #[inline]
    pub fn device_address(&self) -> &vk::DeviceAddress {
        &self.device_address
    }
}

impl Drop for Buffer {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.device.allocator().destroy_buffer(self.buffer, self.allocation)
        }
    }
}