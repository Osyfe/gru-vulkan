use super::*;

impl Swapchain
{
    #[inline]
    pub fn debug_get_indices(&self, index: &SwapchainObjectIndex) -> (usize, usize)
    {
        (index.index, self.cycle_index.get())
    }
}

impl<'a, 'b> CommandBufferRecord<'a, 'b>
{
    #[inline]
    pub fn debug_insert_label(&mut self, label: &str) -> &mut Self
    {
        #[cfg(debug_assertions)]
        if let Some(debug_utils) = &self.buffer.pool.device.debug_utils
        {
            let cstr = std::ffi::CString::new(label).unwrap();
            let label = vk::DebugUtilsLabelEXT::default()
                .label_name(&cstr)
                .color([0.0, 0.0, 0.0, 1.0]);
            unsafe { debug_utils.cmd_insert_debug_utils_label(self.buffer.command_buffer, &label); }
        }
        self
    }
}

impl<'a, 'b, 'c> CommandBufferRecordRenderPass<'a, 'b, 'c>
{
    #[inline]
    pub fn debug_insert_label(&mut self, label: &str) -> &mut Self
    {
        self.record.debug_insert_label(label);
        self
    }
}

trait NameableObject: Sized
{
    type Handle: vk::Handle;
    fn vk_handle(&self) -> Self::Handle;
    fn device(&self) -> &RawDevice;
    fn debug_named(self, name: &str) -> Self
    {
        #[cfg(debug_assertions)]
        if let Some(debug_utils) = &self.device().debug_utils
        {
            let cstr = std::ffi::CString::new(name).unwrap();
            let name = vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(self.vk_handle())
                .object_name(&cstr);
            unsafe { debug_utils.set_debug_utils_object_name(&name) }.unwrap();
        }
        self
    }
}

macro_rules! impl_nameable
{
    ($ty: ident $(<$($lt: lifetime),+>)?, $handle: ty, $field: ident, $($device: tt).+) =>
    {
        impl$(<$($lt),+>)? NameableObject for $ty$(<$($lt),+>)?
        {
            type Handle = $handle;
            fn vk_handle(&self) -> Self::Handle { self.$field }
            fn device(&self) -> &RawDevice { &self.$($device).+ }
        }

        impl$(<$($lt),+>)? $ty$(<$($lt),+>)?
        {
            pub fn debug_named(self, name: &str) -> Self
            {
                <Self as NameableObject>::debug_named(self, name)
            }
        }
    };
}

impl_nameable!(Buffer, vk::Buffer, buffer, device);
impl_nameable!(Image, vk::Image, image, device);
impl_nameable!(DescriptorSet, vk::DescriptorSet, descriptor_set, pool.device);
impl_nameable!(RenderPass, vk::RenderPass, render_pass, device);
impl_nameable!(Pipeline, vk::Pipeline, pipeline, device);
impl_nameable!(Compute, vk::Pipeline, compute, device);
impl_nameable!(CommandBuffer<'a>, vk::CommandBuffer, command_buffer, pool.device);
