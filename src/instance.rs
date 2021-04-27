use super::*;
use std::os::raw::c_char;

fn extension_name_pointers(window: &dyn raw_window_handle::HasRawWindowHandle) -> Vec<*const c_char>
{
    let mut extension_name_pointers = vec![];
    ash_window::enumerate_required_extensions(window).unwrap().iter().for_each(|extension| extension_name_pointers.push(extension.as_ptr()));
    if DEBUG_MODE { extension_name_pointers.push(ash::extensions::ext::DebugUtils::name().as_ptr()); }
    extension_name_pointers
}

fn layer_name_pointers() -> (Vec<std::ffi::CString>, Vec<*const c_char>)
{
    let layer_names: Vec<std::ffi::CString> = if DEBUG_MODE
    {
        vec!
        [
            std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
            //std::ffi::CString::new("VK_LAYER_LUNARG_api_dump").unwrap(),
            std::ffi::CString::new("VK_LAYER_LUNARG_monitor").unwrap()
        ]
    } else { vec![] };
    let layer_name_pointers = layer_names.iter().map(|layer_name| layer_name.as_ptr()).collect();
    (layer_names, layer_name_pointers)
}

fn surface(entry: &ash::Entry, instance: &ash::Instance, window: &dyn raw_window_handle::HasRawWindowHandle) -> vk::SurfaceKHR
{
    unsafe { ash_window::create_surface(entry, instance, window, None) }.unwrap()
}

//the debug callback
unsafe extern "system" fn vulkan_debug_utils_callback
(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32
{
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    vk::FALSE
}

impl Instance
{
    pub fn new(window: &dyn raw_window_handle::HasRawWindowHandle) -> Self
    {
        let entry = unsafe { ash::Entry::new() }.unwrap();
       
        let enginename = std::ffi::CString::new("Osyfe Game Engine").unwrap();
        let app_name = std::ffi::CString::new("Osyfe App").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_version(0, 0, 1))
            .engine_name(&enginename)
            .engine_version(vk::make_version(0, 1, 0))
            .api_version(vk::make_version(1, 0, 106));
            
        let (_layer_names, layer_name_pointers) = layer_name_pointers();
        let extension_name_pointers = extension_name_pointers(window);
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_name_pointers)
            .enabled_extension_names(&extension_name_pointers);
            
        let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity
            (
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
              //| vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
              //| vk::DebugUtilsMessageSeverityFlagsEXT::INFO
              | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            ).message_type
            (
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
              | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
              | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            ).pfn_user_callback(Some(vulkan_debug_utils_callback));
        let instance_create_info = if DEBUG_MODE { instance_create_info.push_next(&mut debug_create_info) } else { instance_create_info };
      
        let instance = unsafe { entry.create_instance(&instance_create_info, None).unwrap() };
    
        let debug_utils = if DEBUG_MODE
        {
            let debug_utils = DebugUtils::new(&entry, &instance);
            let debug_utils_messenger = unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None).unwrap() };
            Some
            ((
                debug_utils,
                debug_utils_messenger
            ))
        } else { None };
        
        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = surface(&entry, &instance, window);
        
        let instance = Instance { _entry: entry, debug: debug_utils, instance, surface_loader, surface };
        instance
    }
    
    pub fn physical_devices(&self) -> Vec<PhysicalDevice>
    {
        let physical_devices = unsafe { self.instance.enumerate_physical_devices() }.unwrap();
        physical_devices.iter().map(|physical_device|
        {
            let physical_device_properties = unsafe { self.instance.get_physical_device_properties(*physical_device) };
            let queue_family_properties = unsafe { self.instance.get_physical_device_queue_family_properties(*physical_device) }
                .iter().enumerate().map
                (|(index, queue_family_properties)| QueueFamilyInfo
                {
                    index,
                    queue_family_properties: *queue_family_properties,
                    surface_support: unsafe { &self.surface_loader.get_physical_device_surface_support(*physical_device, index as u32, self.surface) }.unwrap()
                }).collect();
            PhysicalDevice
            {
                physical_device: *physical_device,
                physical_device_properties,
                queue_family_properties
            }
        }).collect()
    }
    
    pub fn logical_device(self, physical_device: &PhysicalDevice, queues: Vec<(&QueueFamilyInfo, Vec<f32>)>) -> Device
    {
        let PhysicalDevice { physical_device, physical_device_properties: _, queue_family_properties } = physical_device;
        let queue_infos = &queues.iter().map(|(queue_family_info, priorities)|
        {
            let index = queue_family_info.index;
            if priorities.len() > queue_family_properties[index].count() as usize { panic!("Instance::logical_device: Too many requested queues in queue family {}", index); }
            if let Some(priority) = priorities.iter().find(|priority| *priority < &0f32 || *priority > &1f32) { panic!("Instance::logical_device: Invalid priority {}", priority); }
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(index as u32)
                .queue_priorities(&priorities)
                .build()
        }).collect::<Vec<vk::DeviceQueueCreateInfo>>()[..];        
        let device_extension_name_pointers: Vec<*const c_char> = vec![ash::extensions::khr::Swapchain::name().as_ptr()];     
        let (_layer_names, layer_name_pointers) = layer_name_pointers();
        let features = unsafe { self.instance.get_physical_device_features(*physical_device) };
        if features.sampler_anisotropy != 1 { println!("sampler_anisotropy not supported!"); }
        if features.fill_mode_non_solid != 1 { println!("fill_mode_non_solid not supported!"); }
        if features.wide_lines != 1 { println!("wide_lines not supported!"); }
        if features.large_points != 1 { println!("large_points not supported!"); }
        if features.sample_rate_shading != 1 { println!("sample_rate_shading not supported!"); }
        let physical_device_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true)
            .wide_lines(true)
            .sample_rate_shading(true);
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extension_name_pointers)
            .enabled_layer_names(&layer_name_pointers)
            .enabled_features(&physical_device_features);
        let logical_device = unsafe { self.instance.create_device(*physical_device, &device_create_info, None).unwrap() };
        
        let queue_families = queues.iter().map(|(queue_family_info, priorities)|
        {
            let index = queue_family_info.index;
            let queues = priorities.iter().enumerate().map
            (|(queue_index, _priority)|
                Arc::new(Mutex::new(Queue { index, queue: unsafe { logical_device.get_device_queue(index as u32, queue_index as u32) } }))
            ).collect();
            let surface_support = queue_family_properties[index].surface_support;
            let flags = queue_family_properties[index].queue_family_properties.queue_flags.clone();
            QueueFamily { index, queues, flags, surface_support }
        }).collect();

        let allocator_create_info = vk_mem::AllocatorCreateInfo
        {
            physical_device: *physical_device,
            device: logical_device.clone(),
            instance: self.instance.clone(),
            //TODO vk_mem 0.2.3 tries to fix the default bug
            flags: vk_mem::AllocatorCreateFlags::NONE,
            preferred_large_heap_block_size: 0,
            frame_in_use_count: 0,
            heap_size_limits: None
        };
        let allocator = vk_mem::Allocator::new(&allocator_create_info).unwrap();

        Device(Arc::new(RawDevice
        {
            instance: self,
            physical_device: *physical_device,
            logical_device,
            allocator,
            queue_families,
            buffer_layout_count: std::sync::Mutex::new(0)
        }))
    }
}

impl PhysicalDevice
{
    pub fn name(&self) -> String
    {
        let pointer = self.physical_device_properties.device_name.as_ptr();
        String::from(unsafe { std::ffi::CStr::from_ptr(pointer) }.to_str().unwrap())
    }
    
    pub fn queue_families(&self) -> &Vec<QueueFamilyInfo> { &self.queue_family_properties }
}

impl QueueFamilyInfo
{
    pub fn count(&self) -> u32 { self.queue_family_properties.queue_count }
    pub fn supports_graphics(&self) -> bool { self.queue_family_properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) }
    //pub fn supports_compute(&self) -> bool { self.queue_family_properties.queue_flags.contains(vk::QueueFlags::COMPUTE) }
    pub fn supports_transfer(&self) -> bool { self.queue_family_properties.queue_flags.contains(vk::QueueFlags::TRANSFER) }
    //pub fn supports_sparse_binding(&self) -> bool { self.queue_family_properties.queue_flags.contains(vk::QueueFlags::SPARSE_BINDING) }
    pub fn supports_surface(&self) -> bool { self.surface_support }
}
