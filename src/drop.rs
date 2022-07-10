use super::*;

impl Drop for Instance
{
    fn drop(&mut self)
    {
        unsafe
        {
            if let Some(surface) = &self.surface { surface.loader.destroy_surface(surface.surface, None); }
            if let Some((debug_utils, debug_utils_messenger)) = &self.debug { debug_utils.destroy_debug_utils_messenger(*debug_utils_messenger, None); }
            self.instance.destroy_instance(None);
        }
    }
}

impl Drop for RawDevice
{
    fn drop(&mut self)
    {
        unsafe
        {
            std::mem::drop(self.allocator.take());
            self.logical_device.destroy_device(None);
        }
    }
}

impl Drop for Swapchain
{
    fn drop(&mut self)
    {
        unsafe
        {
            for iv in self.swapchain_image_views.iter() { self.device.logical_device.destroy_image_view(*iv, None); }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        } 
    }
}

impl Drop for Buffer
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_buffer(self.buffer, None); }
        self.device.allocator.as_ref().unwrap().lock().unwrap().free(self.allocation.take().unwrap()).unwrap();
    }
}

impl Drop for Image
{
    fn drop(&mut self)
    {
        unsafe
        {
            self.device.logical_device.destroy_image_view(self.image_view, None);
            self.device.logical_device.destroy_image(self.image, None);
        }
        self.device.allocator.as_ref().unwrap().lock().unwrap().free(self.allocation.take().unwrap()).unwrap();
    }
}

impl Drop for ImageBuffer
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_buffer(self.buffer, None); }
        self.device.allocator.as_ref().unwrap().lock().unwrap().free(self.allocation.take().unwrap()).unwrap();
    }
}

impl Drop for Sampler
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_sampler(self.sampler, None); }
    }
}

impl Drop for DescriptorPool
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_descriptor_pool(self.pool, None); }
    }
}

impl Drop for RawDescriptorSetLayout
{
    fn drop(&mut self)
    {
        unsafe
        {
            self.device.logical_device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

impl Drop for Framebuffer
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_framebuffer(self.framebuffer, None); }
    }
}

impl Drop for RenderPass
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_render_pass(self.render_pass, None); }
    }
}

impl Drop for PipelineLayout
{
    fn drop(&mut self)
    {
        unsafe
        {
            self.device.logical_device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

impl Drop for Pipeline
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_pipeline(self.pipeline, None); }
    }
}

impl Drop for CommandPool
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_command_pool(self.pool, None); }
    }
}

impl Drop for CommandBuffer<'_>
{
    fn drop(&mut self)
    {
        unsafe { self.pool.device.logical_device.free_command_buffers(self.pool.pool, &[self.command_buffer]); }
    }
}

impl Drop for Semaphore
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_semaphore(self.semaphore, None); }
    }
}

impl Drop for Fence
{
    fn drop(&mut self)
    {
        unsafe { self.device.logical_device.destroy_fence(self.fence, None); }
    }
}
