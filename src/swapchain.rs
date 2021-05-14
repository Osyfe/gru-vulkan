use super::*;

impl Device
{
    pub fn new_swapchain(&self, old_swapchain: Option<Swapchain>, (width, height): (u32, u32), v_sync: bool) -> Swapchain
    {
        std::mem::drop(old_swapchain);
        let surface = self.0.instance.surface.as_ref().expect("Device::new_swapchain: A swapchain needs a surface.");
        let surface_loader = &surface.loader;
        let surface = &surface.surface;
        let surface_capabilities = unsafe { surface_loader.get_physical_device_surface_capabilities(self.0.physical_device, *surface) }.unwrap();
        let present_modes = unsafe { surface_loader.get_physical_device_surface_present_modes(self.0.physical_device, *surface) }.unwrap();
        let v_sync_mode =
            if present_modes.iter().any(|mode| *mode == vk::PresentModeKHR::FIFO_RELAXED) { vk::PresentModeKHR::FIFO_RELAXED }
            else { vk::PresentModeKHR::FIFO };
        let no_v_sync_mode =
            if present_modes.iter().any(|mode| *mode == vk::PresentModeKHR::IMMEDIATE) { vk::PresentModeKHR::IMMEDIATE }
            else if present_modes.iter().any(|mode| *mode == vk::PresentModeKHR::MAILBOX) { vk::PresentModeKHR::MAILBOX }
            else { vk::PresentModeKHR::FIFO };
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(*surface)
            .min_image_count(3.max(surface_capabilities.min_image_count).min(surface_capabilities.max_image_count))
            .image_format(Swapchain::IMAGE_CHANNEL_TYPE.vk_format())
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(if v_sync { v_sync_mode } else { no_v_sync_mode });
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&self.0.instance.instance, &self.0.logical_device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None).unwrap() };
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();
        let swapchain_image_views = swapchain_images.iter().map(|image|
        {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(Swapchain::IMAGE_CHANNEL_TYPE.vk_format())
                .subresource_range(*subresource_range);
            unsafe { self.0.logical_device.create_image_view(&image_view_create_info, None) }.unwrap()
        }).collect();
        let count = swapchain_images.len();

        Swapchain
        {
            device: self.0.clone(),
            width, height,
            swapchain_loader, swapchain,
            swapchain_images, swapchain_image_views,
            count, cycle_index: std::cell::Cell::new(0)
        }
    }
}

impl Swapchain
{
    pub const IMAGE_CHANNEL_TYPE: ImageChannelType = ImageChannelType::BgraSrgb; //assumes linear shader input

    #[inline]
    pub fn acquire_next_image(&self, signal: Option<&Semaphore>, mark: Option<&Fence>) -> Result<SwapchainObjectIndex, ()>
    {
        let semaphore = match signal
        {
            Some(semaphore) => semaphore.semaphore,
            None => vk::Semaphore::null()
        };
        let fence = match mark
        {
            Some(fence) => fence.fence,
            None => vk::Fence::null()
        };
        match unsafe { self.swapchain_loader.acquire_next_image(self.swapchain, std::u64::MAX, semaphore, fence) }
        {
            Ok((image_index, suboptimal)) => if !suboptimal { Ok(SwapchainObjectIndex { index: image_index as usize }) } else { Err(()) },
            Err(_) => Err(())
        }

    }

    #[inline]
    pub fn present(&self, index: SwapchainObjectIndex, queue: &Queue, wait: &Semaphore) -> bool
    {
        self.cycle_index.set((self.cycle_index.get() + 1) % self.count);
        let semaphores = &[wait.semaphore];
        let swapchains = &[self.swapchain];
        let image_indices = &[index.index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);
        unsafe { self.swapchain_loader.queue_present(queue.queue, &present_info) }.ok().is_some()
    }

    pub fn get_image<'a>(&'a self, index: &'a SwapchainObjectIndex) -> SwapchainImage<'a>
    {
        SwapchainImage
        {
            image: &self.swapchain_images[index.index],
            image_view: &self.swapchain_image_views[index.index],
            width: self.width,
            height: self.height
        }
    }

    pub fn new_objects<T>(&self, constructor: &mut dyn FnMut(&SwapchainObjectIndex) -> T) -> SwapchainObjects<T>
    {
        SwapchainObjects { objects: (0..self.count).map(|index| constructor(&SwapchainObjectIndex { index })).collect() }
    }

    pub fn new_cycle<T>(&self, constructor: &mut dyn Fn() -> T) -> SwapchainCycle<T>
    {
        SwapchainCycle { objects: (0..self.count).map(|_| constructor()).collect() }
    }
}

pub struct SwapchainObjectIndex
{
    index: usize
}

pub struct SwapchainObjects<T>
{
    objects: Vec<T>
}

impl<T> SwapchainObjects<T>
{
    #[inline]
    pub fn get(&self, index: &SwapchainObjectIndex) -> &T
    {
        &self.objects[index.index]
    }

    #[inline]
    pub fn get_mut(&mut self, index: &SwapchainObjectIndex) -> &mut T
    {
        &mut self.objects[index.index]
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<T>
    {
        self.objects.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T>
    {
        self.objects.iter_mut()
    }
}

pub struct SwapchainCycle<T>
{
    objects: Vec<T>
}

impl<T> SwapchainCycle<T>
{
    #[inline]
    pub fn get(&self, swapchain: &Swapchain) -> &T
    {
        &self.objects[swapchain.cycle_index.get()]
    }

    #[inline]
    pub fn get_mut(&mut self, swapchain: &Swapchain) -> &mut T
    {
        &mut self.objects[swapchain.cycle_index.get()]
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<T>
    {
        self.objects.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T>
    {
        self.objects.iter_mut()
    }
}