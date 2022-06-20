use std::sync::Arc;

use anyhow::Result;
use ash::vk;
use raw_window_handle::HasRawWindowHandle;

use crate::backend::Instance;

pub struct Surface {
    surface: vk::SurfaceKHR,
    instance: Arc<Instance>
}

impl Surface {
    pub fn new(instance: Arc<Instance>, window: &impl HasRawWindowHandle) -> Result<Arc<Self>> {
        unsafe {
            let surface = ash_window::create_surface(instance.entry_loader(), instance.loader(), window, None)?;
            Ok(Arc::new(Self { surface, instance }))
        }
    }

    #[inline]
    pub fn surface(&self) -> &vk::SurfaceKHR {
        &self.surface
    }
}

impl Drop for Surface {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.instance.surface_loader().destroy_surface(self.surface, None);
        }
    }
}
