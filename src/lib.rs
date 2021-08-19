pub const DEBUG_MODE: bool = cfg!(debug_assertions);

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use ash_window;
use ash::{self, vk, extensions::ext::DebugUtils};

pub use gru_vulkan_derive::{VertexAttributeGroupReprCpacked, InstanceAttributeGroupReprCpacked, DescriptorStructReprC};
pub use inline_spirv::include_spirv;

mod drop;
mod instance;
mod swapchain;
mod device;
mod render_pass;
mod pipeline;
mod buffer;
mod image;
mod descriptor;
mod command;
pub use instance::*;
pub use swapchain::*;
pub use device::*;
pub use render_pass::*;
pub use pipeline::*;
pub use buffer::*;
pub use image::*;
pub use descriptor::*;
pub use command::*;

//     #####     INSTANCE     #####

struct Surface
{
    loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR
}

pub struct Instance
{
    _entry: ash::Entry,
    debug: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
    instance: ash::Instance,
    surface: Option<Surface>
}

pub struct PhysicalDevice
{
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_family_properties: Vec<QueueFamilyInfo>
}

#[derive(Clone)]
pub struct QueueFamilyInfo
{
    index: usize,
    queue_family_properties: vk::QueueFamilyProperties,
    surface_support: bool
}

//     #####     DEVICE     #####

pub struct Queue
{
    index: usize,
    queue: vk::Queue
}

pub struct QueueFamily
{
    index: usize,
    queues: Vec<Arc<Mutex<Queue>>>,
    flags: vk::QueueFlags,
    surface_support: bool
}

struct RawDevice
{
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    logical_device: ash::Device,
    allocator: vk_mem::Allocator,
    queue_families: Vec<QueueFamily>,
    buffer_layout_count: std::sync::Mutex<u32>
}

#[derive(Clone)]
pub struct Device(Arc<RawDevice>);

pub struct Swapchain
{
    device: Arc<RawDevice>,
    width: u32,
    height: u32,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    count: usize,
    cycle_index: std::cell::Cell<usize>
}

pub struct SwapchainImage<'a>
{
    image: &'a vk::Image,
    image_view: &'a vk::ImageView,
    width: u32,
    height: u32
}

//     #####     BUFFER     #####

pub trait IndexType
{
    const FORMAT: vk::IndexType;
}

pub enum InputRate
{
    VERTEX,
    INSTANCE
}

#[repr(transparent)]
pub struct AttributeLocation(pub u32);

pub enum AttributeType
{
    F1,
    F2,
    F3,
    F4
}

pub trait AttributeGroupReprCpacked
{
    const RATE: InputRate;
    const ATTRIBUTES: &'static [(AttributeLocation, AttributeType)];
}

pub struct AttributeGroupInfo
{
    rate: InputRate,
    attributes: &'static [(AttributeLocation, AttributeType)],
}

pub struct BufferType
{
    id: u32,
    offset_in_bytes: u64,
    uniform_align: u64,
    indices: bool,
    attributes: bool,
    uniforms: bool,
    sealed: bool
}

pub struct BufferView<T>
{
    layout_id: u32,
    offset_in_bytes: usize,
    count: u32,
    stride: u32,
    phantom: PhantomData<T>
}

pub struct Buffer
{
    device: Arc<RawDevice>,
    allocation: vk_mem::Allocation,
    _allocation_info: vk_mem::AllocationInfo,
    buffer: vk::Buffer,
    buffer_usage: BufferUsage,
    layout_id: u32,
    size_in_bytes: u64
}

//     #####     IMAGE     #####

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageLayout
{
    Undefined,
    Attachment,
    Shader
}
//3 channel images take the same space as 4 channels, therefore we do not support those
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageChannelType
{
    BgraSrgb,
    BgraSnorm,
    BgraUnorm,
    BgraSint,
    BgraUint,
    RSrgb,
    RSnorm,
    RUnorm,
    RSint,
    RUint,
    DSfloat
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ImageType
{
    pub channel: ImageChannelType,
    pub width: u32,
    pub height: u32,
    pub layers: Option<u32>
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Msaa
{
    None,
    X2,
    X4
}

#[derive(Clone, Copy)]
pub enum ImageUsage
{
    Texture { mipmapping: bool },
    Attachment { depth: bool, samples: Msaa, texture: bool, transfer_src: bool }
}

pub struct Image
{
    device: Arc<RawDevice>,
    allocation: vk_mem::Allocation,
    _allocation_info: vk_mem::AllocationInfo,
    image: vk::Image,
    image_view: vk::ImageView,
    image_type: ImageType,
    mip_levels: u32,
    image_usage: ImageUsage
}

pub struct ImageBuffer
{
    device: Arc<RawDevice>,
    allocation: vk_mem::Allocation,
    _allocation_info: vk_mem::AllocationInfo,
    buffer: vk::Buffer,
    image_type: ImageType
}

pub struct Sampler
{
    device: Arc<RawDevice>,
    sampler: vk::Sampler
}

//     #####     DESCRIPTOR     #####

pub trait DescriptorStructReprC: Copy { }

#[derive(PartialEq, Eq)]
pub enum DescriptorBindingType
{
    Struct { size_in_bytes: u32 },
    Sampler { image_channel_type: ImageChannelType },
    SubpassInput { image_channel_type: ImageChannelType }
}

#[derive(PartialEq, Eq)]
pub struct DescriptorBindingInfo
{
    ty: DescriptorBindingType,
    count: u32,
    vertex: bool,
    fragment: bool
}

struct RawDescriptorSetLayout
{
    device: Arc<RawDevice>,
    set: u32,
    bindings: Vec<DescriptorBindingInfo>,
    descriptor_set_layout: vk::DescriptorSetLayout
}

#[derive(Clone)]
pub struct DescriptorSetLayout(std::rc::Rc<RawDescriptorSetLayout>);

struct DescriptorPool
{
    device: Arc<RawDevice>,
    pool: vk::DescriptorPool
}

pub struct DescriptorSet
{
    pool: std::rc::Rc<DescriptorPool>,
    descriptor_set: vk::DescriptorSet,
    layout: std::rc::Rc<RawDescriptorSetLayout>
}

//     #####     RENDER STUFF     #####

pub struct Framebuffer
{
    device: Arc<RawDevice>,
    framebuffer: vk::Framebuffer,
    size: (u32, u32)
}

pub struct RenderPass
{
    device: Arc<RawDevice>,
    render_pass: vk::RenderPass,
    clear_values: Vec<vk::ClearValue>
}

pub struct PipelineLayout
{
    device: Arc<RawDevice>,
    layout: vk::PipelineLayout
}

pub struct Pipeline
{
    device: Arc<RawDevice>,
    pipeline: vk::Pipeline
}

pub struct IndexBinding<'a>
{
    buffer: &'a Buffer,
    offset_in_bytes: u64,
    format: vk::IndexType
}

pub struct AttributeBinding<'a>
{
    buffer: &'a Buffer,
    offset_in_bytes: u64
}

pub struct CommandPool
{
    device: Arc<RawDevice>,
    pool: vk::CommandPool,
    queue_family_index: usize,
    queue_family_flags: vk::QueueFlags
}

pub struct CommandBuffer<'a>
{
    pool: &'a CommandPool,
    command_buffer: vk::CommandBuffer
}

pub struct Semaphore
{
    device: Arc<RawDevice>,
    semaphore: vk::Semaphore
}

pub struct Fence
{
    device: Arc<RawDevice>,
    fence: vk::Fence
}
