#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

const DEBUG_MODE: bool = cfg!(debug_assertions);

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
mod debug;
pub use instance::*;
pub use swapchain::*;
//pub use device::*;
pub use render_pass::*;
pub use pipeline::*;
pub use buffer::*;
pub use image::*;
//pub use descriptor::*;
pub use command::*;
//pub use debug::*;

use std::{marker::PhantomData, sync::{Arc, Mutex}, rc::Rc};
use ash::{self, vk};
use gpu_allocator::vulkan as alloc;

pub use gru_vulkan_derive::{VertexAttributeGroupReprCpacked, InstanceAttributeGroupReprCpacked, StorageStructReprC, DescriptorStructReprC};
pub use inline_spirv::include_spirv;

//     #####     INSTANCE     #####

struct Surface
{
    loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR
}

pub struct Instance
{
    #[allow(unused)] entry: ash::Entry,
    debug: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    instance: ash::Instance,
    surface: Option<Surface>
}

pub struct PhysicalDevice
{
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_family_properties: Box<[QueueFamilyInfo]>
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
    queues: Box<[Arc<Mutex<Queue>>]>,
    flags: vk::QueueFlags,
    surface_support: bool
}

struct Props
{
    min_uniform_buffer_offset_alignment: u64,
    min_storage_buffer_offset_alignment: u64
}

struct RawDevice
{
    instance: Instance,
    debug_utils: Option<ash::ext::debug_utils::Device>,
    physical_device: vk::PhysicalDevice,
    props: Props,
    logical_device: ash::Device,
    allocator: Option<Mutex<alloc::Allocator>>,
    queue_families: Box<[QueueFamily]>,
    buffer_layout_count: std::sync::atomic::AtomicU32
}

#[derive(Clone)]
pub struct Device(Arc<RawDevice>);

pub struct Swapchain
{
    device: Arc<RawDevice>,
    width: u32,
    height: u32,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Box<[vk::Image]>,
    swapchain_image_views: Box<[vk::ImageView]>,
    count: usize,
    cycle_index: std::cell::Cell<usize>
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
pub enum InputRate
{
    Vertex,
    Instance
}

#[repr(transparent)]
pub struct AttributeLocation(pub u32);

#[derive(Clone, Copy)]
pub enum AttributeType
{
    F1, F2, F3, F4,
    I1, I2, I3, I4,
    U1, U2, U3, U4
}

pub trait AttributeGroupReprCpacked
{
    const RATE: InputRate;
    const ATTRIBUTES: &'static [(AttributeLocation, AttributeType)];
}

#[derive(Clone, Copy)]
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
    storage_align: u64,
    indices: bool,
    attributes: bool,
    uniforms: bool,
    storage: bool
}

pub struct BufferTypeBuilder(BufferType);

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
    allocation: Option<alloc::Allocation>,
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
    RgbaSfloat,
    RgbaShalffloat,
    RSrgb,
    RSnorm,
    RUnorm,
    RSint,
    RUint,
    R32Uint,
    RSfloat,
    DSfloat
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageLayers
{
    Single, //sampler2D
    Array(u32), //sampler2DArray
    Cube, //samplerCubeMap
    CubeArray(u32) //samplerCubeMapArray
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ImageType
{
    pub channel: ImageChannelType,
    pub width: u32,
    pub height: u32,
    pub layers: ImageLayers
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Msaa
{
    None,
    X2,
    X4,
    X8
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
    allocation: Option<alloc::Allocation>,
    image: vk::Image,
    image_view: vk::ImageView,
    image_type: ImageType,
    mip_levels: u32,
    image_usage: ImageUsage
}

pub struct ImageBuffer
{
    device: Arc<RawDevice>,
    allocation: Option<alloc::Allocation>,
    buffer: vk::Buffer,
    image_type: ImageType
}

pub struct Sampler
{
    device: Arc<RawDevice>,
    sampler: vk::Sampler
}

//     #####     DESCRIPTOR     #####

pub trait StorageStructReprC { }
pub trait DescriptorStructReprC: Copy { }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DescriptorBindingType
{
    Storage,
    Struct { size_in_bytes: u32 },
    Sampler { image_channel_type: ImageChannelType },
    SubpassInput { image_channel_type: ImageChannelType }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DescriptorVisibility
{
    pub compute: bool,
    pub vertex: bool,
    pub fragment: bool
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DescriptorBindingInfo
{
    ty: DescriptorBindingType,
    count: u32,
    visibility: DescriptorVisibility
}

struct RawDescriptorSetLayout
{
    device: Arc<RawDevice>,
    set: u32,
    bindings: Box<[DescriptorBindingInfo]>,
    descriptor_set_layout: vk::DescriptorSetLayout
}

#[derive(Clone)]
pub struct DescriptorSetLayout(Arc<RawDescriptorSetLayout>);

struct DescriptorPool
{
    device: Arc<RawDevice>,
    pool: vk::DescriptorPool
}

pub struct DescriptorSet
{
    pool: Arc<DescriptorPool>,
    descriptor_set: vk::DescriptorSet,
    layout: Arc<RawDescriptorSetLayout>
}

//     #####     RENDER STUFF     #####

pub struct Framebuffer
{
    device: Arc<RawDevice>,
    image_views: Vec<vk::ImageView>,
    framebuffer: vk::Framebuffer,
    size: (u32, u32)
}

pub struct RenderPass
{
    device: Arc<RawDevice>,
    render_pass: vk::RenderPass,
    clear_values: Box<[vk::ClearValue]>
}

pub struct PipelineLayout
{
    device: Arc<RawDevice>,
    layout: vk::PipelineLayout,
    push_constant: Option<(vk::ShaderStageFlags, u32)>
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy)]
pub struct ViewInfo
{
    pub viewport_origin: (f32, f32),
    pub viewport_size: (f32, f32),
    pub scissor_origin: (i32, i32),
    pub scissor_size: (u32, u32)
}

pub struct Pipeline
{
    device: Arc<RawDevice>,
    pipeline: vk::Pipeline
}

#[derive(Clone, Copy)]
pub struct IndexBinding<'a>
{
    buffer: &'a Buffer,
    offset_in_bytes: u64,
    format: vk::IndexType
}

#[derive(Clone, Copy)]
pub struct AttributeBinding<'a>
{
    buffer: &'a Buffer,
    offset_in_bytes: u64
}

//     #####     COMPUTE STUFF     #####

pub struct Compute
{
    device: Arc<RawDevice>,
    compute: vk::Pipeline
}

struct RawCommandPool
{
    device: Arc<RawDevice>,
    pool: vk::CommandPool,
    queue_family_index: usize,
    queue_family_flags: vk::QueueFlags
}

pub struct CommandPool
{
    pool: Rc<RawCommandPool>
}

pub struct CommandBuffer
{
    pool: Rc<RawCommandPool>,
    command_buffer: vk::CommandBuffer
}

#[derive(Clone, Copy)]
pub enum WaitStage
{
    None,
    VertexInput,
    ColorOutput,
    Compute
}

pub struct Semaphore
{
    device: Arc<RawDevice>,
    semaphore: vk::Semaphore,
    wait_stage: vk::PipelineStageFlags
}

pub struct Fence
{
    device: Arc<RawDevice>,
    fence: vk::Fence
}

pub struct CopyFence<'a, 'b>
{
    pub mark: Fence,
    pub command_buffer: CommandBuffer,
    _src: &'a (),
    _dst: &'b ()
}
