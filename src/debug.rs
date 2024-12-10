use super::*;

impl<'a, 'b> CommandBufferRecord<'a, 'b>
{
    #[inline]
    pub fn insert_debug_label(&mut self, label: &str) -> &mut Self
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
    pub fn insert_debug_label(&mut self, label: &str) -> &mut Self
    {
        self.record.insert_debug_label(label);
        self
    }
}

pub trait NameableObject: Sized
{
    type Handle: vk::Handle;
    fn vk_handle(&self) -> Self::Handle;
    fn named(self, device: &Device, name: &str) -> Self
    {
        #[cfg(debug_assertions)]
        if let Some(debug_utils) = &device.0.debug_utils
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
    ($ty: ident $(<$($lt: lifetime),+>)?, $handle: ty, $field: ident) =>
    {
        impl$(<$($lt),+>)? NameableObject for $ty$(<$($lt),+>)?
        {
            type Handle = $handle;
            fn vk_handle(&self) -> Self::Handle
            {
                self.$field
            }
        }
    };
}

impl_nameable!(Buffer, vk::Buffer, buffer);
impl_nameable!(Image, vk::Image, image);
impl_nameable!(DescriptorSet, vk::DescriptorSet, descriptor_set);
impl_nameable!(RenderPass, vk::RenderPass, render_pass);
impl_nameable!(Pipeline, vk::Pipeline, pipeline);
impl_nameable!(Compute, vk::Pipeline, compute);
impl_nameable!(CommandBuffer<'a>, vk::CommandBuffer, command_buffer);
