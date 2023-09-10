use super::*;
use std::os::raw::c_char;

fn layer_name_pointers(entry: &ash::Entry) -> (Vec<std::ffi::CString>, Vec<*const c_char>)
{
    let available_layers = entry.enumerate_instance_layer_properties().unwrap();
    let layer_names: Vec<std::ffi::CString> = if DEBUG_MODE
    {
        vec!
        [
            std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
            //std::ffi::CString::new("VK_LAYER_LUNARG_api_dump").unwrap(),
            std::ffi::CString::new("VK_LAYER_LUNARG_monitor").unwrap()
        ]
    } else { vec![] };
    let layer_name_pointers = layer_names.iter()
        .filter(|name| available_layers.iter().any(|available| unsafe
        {
            let available = std::ffi::CStr::from_ptr(&available.layer_name as *const c_char);
            let wanted = name.as_c_str();
            available == wanted
        }))
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    (layer_names, layer_name_pointers)
}

fn extension_name_pointers(entry: &ash::Entry, display: Option<&dyn raw_window_handle::HasRawDisplayHandle>) -> (Vec<*const c_char>, bool)
{
    let available_extensions: Vec<_> = entry.enumerate_instance_extension_properties(None).unwrap().into_iter()
        .map(|ext| unsafe { std::ffi::CStr::from_ptr(&ext.extension_name as *const c_char) }.to_owned())
        .collect();
    let exists = move |name: &std::ffi::CString| available_extensions.iter().any(|available| available == name);
    let mut extension_name_pointers = vec![];
    if let Some(display) = display
    {
        ash_window::enumerate_required_extensions(display.raw_display_handle()).unwrap().iter().for_each(|extension|
        {
            let name = unsafe { std::ffi::CStr::from_ptr(*extension as *const c_char) }.to_owned();
            if exists(&name) { extension_name_pointers.push(*extension); }
            else { println!("Instance::new: Extension {name:?} missing"); }
        });
    }
    let mut debug = false;
    if DEBUG_MODE
    {
        let name = ash::extensions::ext::DebugUtils::name();
        if exists(&name.to_owned())
        {
            extension_name_pointers.push(name.as_ptr());
            debug = true;
        }
    }
    (extension_name_pointers, debug)
}

fn surface(entry: &ash::Entry, instance: &ash::Instance, display: &dyn raw_window_handle::HasRawDisplayHandle, window: &dyn raw_window_handle::HasRawWindowHandle) -> vk::SurfaceKHR
{
    unsafe { ash_window::create_surface(entry, instance, display.raw_display_handle(), window.raw_window_handle(), None) }.unwrap()
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

pub unsafe trait HasBothHandles: raw_window_handle::HasRawDisplayHandle + raw_window_handle::HasRawWindowHandle {}
unsafe impl<T: raw_window_handle::HasRawDisplayHandle + raw_window_handle::HasRawWindowHandle> HasBothHandles for T {}

impl Instance
{
    pub fn new<W: HasBothHandles>(window: Option<&W>) -> Self
    {
        #[cfg(not(feature = "loaded"))]
        let entry = ash::Entry::linked();
        #[cfg(feature = "loaded")]
        let entry = unsafe { ash::Entry::load() }.unwrap();
       
        let enginename = std::ffi::CString::new("gru-vulkan").unwrap();
        let app_name = std::ffi::CString::new("osyfe app").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 0, 0, 1))
            .engine_name(&enginename)
            .engine_version(vk::make_api_version(0, 0, 0, 1))
            .api_version(vk::make_api_version(0, 1, 0, 106));
            
        let (_layer_names, layer_name_pointers) = layer_name_pointers(&entry);
        let (extension_name_pointers, debug_ext) = extension_name_pointers(&entry, window.map(|window| window as &dyn raw_window_handle::HasRawDisplayHandle));
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_name_pointers)
            .enabled_extension_names(&extension_name_pointers);
            
        let (instance, debug_utils) = if debug_ext
        {   
            let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity
                (
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                  | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                  //| vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                  //| vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                ).message_type
                (
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                  | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                  | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                ).pfn_user_callback(Some(vulkan_debug_utils_callback));
            let instance_create_info = instance_create_info.push_next(&mut debug_create_info);
            let instance = unsafe { entry.create_instance(&instance_create_info, None).unwrap() };

            let debug_utils = DebugUtils::new(&entry, &instance);
            let debug_utils_messenger = unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None).unwrap() };
            let debug_utils = Some((debug_utils, debug_utils_messenger));
            (instance, debug_utils)
        } else
        {
            let instance = unsafe { entry.create_instance(&instance_create_info, None).unwrap() };
            (instance, None)
        };
        
        let surface = window.map(|window|
        {
            let loader = ash::extensions::khr::Surface::new(&entry, &instance);
            let surface = surface(&entry, &instance, window as &dyn raw_window_handle::HasRawDisplayHandle, window as &dyn raw_window_handle::HasRawWindowHandle);
            Surface { loader, surface }
        });
        
        Self { _entry: entry, debug: debug_utils, instance, surface }
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
                    surface_support: match &self.surface
                    {
                        Some(surface) => unsafe { surface.loader.get_physical_device_surface_support(*physical_device, index as u32, surface.surface) }.unwrap(),
                        None => false
                    }
                }).collect();
            PhysicalDevice
            {
                physical_device: *physical_device,
                physical_device_properties,
                queue_family_properties
            }
        }).collect()
    }
    
    pub fn logical_device<'a, A: AsRef<[f32]>, B: AsRef<[(&'a QueueFamilyInfo, A)]>>(self, physical_device: &PhysicalDevice, queues: B) -> Device
    {
        let PhysicalDevice { physical_device, physical_device_properties: _, queue_family_properties } = physical_device;
        let queue_infos = &queues.as_ref().iter().map(|(queue_family_info, priorities)|
        {
            let priorities = priorities.as_ref();
            let index = queue_family_info.index;
            if priorities.len() == 0 { panic!("Instance::logical_device: No requested queue in queue family {}", index); }
            if priorities.len() > queue_family_properties[index].count() as usize { panic!("Instance::logical_device: Too many requested queues in queue family {}", index); }
            if let Some(priority) = priorities.iter().find(|priority| *priority < &0f32 || *priority > &1f32) { panic!("Instance::logical_device: Invalid priority {}", priority); }
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(index as u32)
                .queue_priorities(&priorities)
                .build()
        }).collect::<Vec<vk::DeviceQueueCreateInfo>>()[..];        
        let device_extension_name_pointers: Vec<*const c_char> = if self.surface.is_some() { vec![ash::extensions::khr::Swapchain::name().as_ptr()] } else { vec![] };     
        //let (_layer_names, layer_name_pointers) = layer_name_pointers(&self.entry);
        let features = unsafe { self.instance.get_physical_device_features(*physical_device) };
        if features.sampler_anisotropy != 1 { println!("sampler_anisotropy not supported!"); }
        if features.fill_mode_non_solid != 1 { println!("fill_mode_non_solid not supported!"); }
        if features.wide_lines != 1 { println!("wide_lines not supported!"); }
        if features.large_points != 1 { println!("large_points not supported!"); }
        if features.sample_rate_shading != 1 { println!("sample_rate_shading not supported!"); }
        let physical_device_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(features.sampler_anisotropy == 1)
            .fill_mode_non_solid(features.fill_mode_non_solid == 1)
            .wide_lines(features.wide_lines == 1)
            .sample_rate_shading(features.sample_rate_shading == 1);
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extension_name_pointers)
            //.enabled_layer_names(&layer_name_pointers)
            .enabled_features(&physical_device_features);
        let logical_device = unsafe { self.instance.create_device(*physical_device, &device_create_info, None).unwrap() };
        
        let queue_families = queues.as_ref().iter().map(|(queue_family_info, priorities)|
        {
            let priorities = priorities.as_ref();
            let index = queue_family_info.index;
            let queues = priorities.iter().enumerate().map
            (|(queue_index, _priority)|
                Arc::new(Mutex::new(Queue { index, queue: unsafe { logical_device.get_device_queue(index as u32, queue_index as u32) } }))
            ).collect();
            let surface_support = queue_family_properties[index].surface_support;
            let flags = queue_family_properties[index].queue_family_properties.queue_flags.clone();
            QueueFamily { index, queues, flags, surface_support }
        }).collect();

        let allocator_create_desc = alloc::AllocatorCreateDesc
        {
            instance: self.instance.clone(),
            device: logical_device.clone(),
            physical_device: *physical_device,
            debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
            buffer_device_address: false
        };
        let allocator = alloc::Allocator::new(&allocator_create_desc).unwrap();

        Device(Arc::new(RawDevice
        {
            instance: self,
            physical_device: *physical_device,
            logical_device,
            allocator: Some(Mutex::new(allocator)),
            queue_families,
            buffer_layout_count: std::sync::atomic::AtomicU32::new(0)
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
    
    pub fn queue_families(&self) -> &[QueueFamilyInfo] { &self.queue_family_properties }
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

impl PartialEq for QueueFamilyInfo
{
    fn eq(&self, other: &Self) -> bool
    {
        self.index == other.index
    }
}
