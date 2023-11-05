use ash::{extensions::khr::Surface, vk};
use glam::Vec2;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use async_winit as winit;
use winit::{event_loop::EventLoop, window::WindowBuilder, ThreadUnsafe};


pub struct Window {
    handle: winit::window::Window<ThreadUnsafe>,
    surface_loader: Option<Surface>,
    surface: Option<vk::SurfaceKHR>,
}

impl Window {
    pub async fn new<S: Into<String>>(
        width: u32,
        height: u32,
        title: S,
    ) -> Self {
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width as f64, height as f64))
            //.with_decorations(false)
            .build().await.unwrap();
        Window {
            handle: window,
            surface_loader: None,
            surface: None,
        }
    }

    pub fn create_surface(&mut self, entry: &ash::Entry, instance: &ash::Instance) {
        self.surface_loader = Some(Surface::new(entry, instance));
        unsafe {
            self.surface =
                Some(ash_window::create_surface(entry, instance, self.handle.raw_display_handle(), self.handle.raw_window_handle(), None).unwrap());
        }
    }

    pub fn handle(&self) -> &winit::window::Window<ThreadUnsafe> {
        &self.handle
    }

    pub fn surface(&self) -> vk::SurfaceKHR {
        self.surface.unwrap()
    }

    pub fn surface_loader(&self) -> &Surface {
        self.surface_loader.as_ref().unwrap()
    }

    pub fn set_title(&mut self, title: &str) {
        self.handle.set_title(title);
    }

    pub unsafe fn get_surface_support(
        &self,
        pdevice: vk::PhysicalDevice,
        queue_index: u32,
    ) -> bool {
        self.surface_loader
            .as_ref()
            .unwrap()
            .get_physical_device_surface_support(pdevice, queue_index, self.surface.unwrap())
            .unwrap()
    }

    pub unsafe fn get_surface_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> vk::SurfaceCapabilitiesKHR {
        self.surface_loader
            .as_ref()
            .unwrap()
            .get_physical_device_surface_capabilities(physical_device, self.surface.unwrap())
            .unwrap()
    }

    pub unsafe fn get_surface_format(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> vk::SurfaceFormatKHR {
        self.surface_loader
            .as_ref()
            .unwrap()
            .get_physical_device_surface_formats(physical_device, self.surface.unwrap())
            .unwrap()[0]
    }

    pub unsafe fn get_surface_present_mode(
        &self,
        physical_device: vk::PhysicalDevice,
        desired: vk::PresentModeKHR,
    ) -> vk::PresentModeKHR {
        let present_modes = self
            .surface_loader
            .as_ref()
            .unwrap()
            .get_physical_device_surface_present_modes(physical_device, self.surface.unwrap())
            .unwrap();
        present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == desired)
            .unwrap_or(vk::PresentModeKHR::FIFO)
    }

    pub async unsafe fn get_surface_extent(&self, physical_device: vk::PhysicalDevice) -> vk::Extent2D {
        let capabilities = self.get_surface_capabilities(physical_device);
        let extent = self.get_extent().await;
        match capabilities.current_extent.width {
            std::u32::MAX => extent,
            _ => capabilities.current_extent,
        }
    }

    pub async fn get_size(&self) -> Vec2 {
        let sz = self.handle.inner_size().await;
        Vec2::new(sz.width as f32, sz.height as f32)
    }

    pub async fn get_width(&self) -> u32 {
        self.handle.inner_size().await.width
    }

    pub async fn get_height(&self) -> u32 {
        self.handle.inner_size().await.height
    }

    pub async fn get_extent(&self) -> vk::Extent2D {
        let sz = self.handle.inner_size().await;
        vk::Extent2D {
            width: sz.width as u32,
            height: sz.height as u32,
        }
    }

    pub async fn get_extent_3d(&self) -> vk::Extent3D {
        let extent = self.get_extent().await;
        vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        }
    }

    pub async fn get_viewport(&self) -> vk::Viewport {
        let sz = self.handle.inner_size().await;
        vk::Viewport::builder()
            .width(sz.width as f32)
            .height(sz.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()
    }

    pub async fn get_viewport_gl(&self) -> vk::Viewport {
        let sz = self.handle.inner_size().await;
        vk::Viewport::builder()
            .x(0.0)
            .y(sz.height as f32)
            .width(sz.width as f32)
            .height(-(sz.height as f32))
            .min_depth(0.0)
            .max_depth(1.0)
            .build()
    }

    pub async fn get_rect(&self) -> vk::Rect2D {
        vk::Rect2D::builder().extent(self.get_extent().await).build()
    }

    pub fn destroy_surface(&mut self) {
        unsafe {
            match self.surface_loader.as_mut() {
                Some(sl) => sl.destroy_surface(self.surface.unwrap(), None),
                None => {}
            }
        }
        self.surface_loader = None;
        self.surface = None;
    }

    pub async fn is_minimized(&self) -> bool {
        let sz = self.handle.inner_size().await;
        sz.width == 0 && sz.height == 0
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.destroy_surface();
    }
}
