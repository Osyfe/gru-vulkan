pub const DEBUG_MODE: bool = cfg!(debug_assertions);

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use ash_window;
use ash::{self, vk, version::{EntryV1_0, InstanceV1_0, DeviceV1_0}, extensions::ext::DebugUtils};

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

pub struct Instance
{
    _entry: ash::Entry,
    debug: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
    instance: ash::Instance,
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR
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
    _swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    count: usize,
    cycle_index: std::cell::Cell<usize>
}

pub struct SwapchainImage<'a>
{
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

pub struct BufferLayout
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
    memory_type: MemoryType,
    transfer_type: TransferType,
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

impl ImageLayout
{
    const fn vk_image_layout(&self, depth: bool) -> vk::ImageLayout
    {
        match self
        {
            ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
            ImageLayout::Attachment => if depth { vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL } else { vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL },
            ImageLayout::Shader => if depth { vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL } else { vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }
        }
    }
}

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

impl ImageChannelType
{
    const fn vk_format(&self) -> vk::Format
    {
        match self
        {
            ImageChannelType::BgraSrgb => vk::Format::B8G8R8A8_SRGB,
            ImageChannelType::BgraSnorm => vk::Format::B8G8R8A8_SNORM,
            ImageChannelType::BgraUnorm => vk::Format::B8G8R8A8_UNORM,
            ImageChannelType::BgraSint => vk::Format::B8G8R8A8_SINT,
            ImageChannelType::BgraUint => vk::Format::B8G8R8A8_UINT,
            ImageChannelType::RSrgb => vk::Format::R8_SRGB,
            ImageChannelType::RSnorm => vk::Format::R8_SNORM,
            ImageChannelType::RUnorm => vk::Format::R8_UNORM,
            ImageChannelType::RSint => vk::Format::R8_SINT,
            ImageChannelType::RUint => vk::Format::R8_UINT,
            ImageChannelType::DSfloat => vk::Format::D32_SFLOAT
        }
    }

    const fn has_depth(&self) -> bool
    {
        match self
        {
            ImageChannelType::DSfloat => true,
            _ => false
        }
    }
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

impl Msaa
{
    const fn vk_sample_count(&self) -> vk::SampleCountFlags
    {
        match self
        {
            Msaa::None => vk::SampleCountFlags::TYPE_1,
            Msaa::X2 => vk::SampleCountFlags::TYPE_2,
            Msaa::X4 => vk::SampleCountFlags::TYPE_4
        }
    }
}

#[derive(Clone, Copy)]
pub enum ImageUsage
{
    Texture { mipmapping: bool },
    Attachment { depth: bool, samples: Msaa, texture: bool }
}

impl ImageUsage
{
    const fn depth(&self) -> bool
    {
        match self
        {
            ImageUsage::Attachment { depth: true, .. } => true,
             _ => false
        }
    }

    const fn vk_sample_count(&self) -> vk::SampleCountFlags
    {
        if let ImageUsage::Attachment { samples, .. } = self { samples.vk_sample_count() }
        else { vk::SampleCountFlags::TYPE_1 }
    }
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
