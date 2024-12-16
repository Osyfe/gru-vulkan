use super::*;

//the debug callback
unsafe extern "system" fn vulkan_debug_utils_callback
(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32
{
    /*
    let names: String = std::slice::from_raw_parts((*p_callback_data).p_objects, (*p_callback_data).object_count as usize)
        .iter()
        .flat_map(|obj|
            if obj.p_object_name.is_null() { None }
            else { Some(std::ffi::CStr::from_ptr(obj.p_object_name).to_str().unwrap()) }
        )
        .intersperse(", ")
        .collect();
    */
    let ty = format!("{:?}", message_type).to_lowercase();
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);

    #[cfg(feature = "log")]
    {
        let level =
            if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) { log::Level::Error }
            else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) { log::Level::Warn }
            else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE) { log::Level::Debug }
            else { log::Level::Trace };
        log::log!(level, "[{ty}] {message:?}");
    }
    
    #[cfg(not(feature = "log"))]
    {
        let severity = format!("{:?}", message_severity).to_lowercase();
        println!("[{}][{}] {:?}", severity, ty, message);
    }

    vk::FALSE
}

pub(crate) fn create_instance(entry: &ash::Entry, instance_create_info: vk::InstanceCreateInfo) -> (ash::Instance, (ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT))
{
    let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity
        (
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
          | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
          | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
          | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
        ).message_type
        (
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
          | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
          | vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
        ).pfn_user_callback(Some(vulkan_debug_utils_callback));
    let instance_create_info = instance_create_info.push_next(&mut debug_create_info);
    let instance = unsafe { entry.create_instance(&instance_create_info, None).unwrap() };

    let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
    let debug_utils_messenger = unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None).unwrap() };
    let debug_utils = (debug_utils, debug_utils_messenger);
    (instance, debug_utils)
}

impl Swapchain
{
    #[inline]
    pub fn debug_get_indices(&self, index: &SwapchainObjectIndex) -> (usize, usize)
    {
        (index.index, self.cycle_index.get())
    }
}

impl<'a> CommandBufferRecord<'a>
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

impl<'a, 'b> CommandBufferRecordRenderPass<'a, 'b>
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
impl_nameable!(CommandBuffer, vk::CommandBuffer, command_buffer, pool.device);
