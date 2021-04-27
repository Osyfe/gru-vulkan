use super::*;
use gru_math::{Vec2, Vec3, Vec4};

//     #####     DATA TYPES     #####

impl IndexType for u16
{
    const FORMAT: vk::IndexType = vk::IndexType::UINT16;
}

impl IndexType for u32
{
    const FORMAT: vk::IndexType = vk::IndexType::UINT32;
}

impl AttributeType
{
    pub(crate) fn vk_format(&self) -> vk::Format
    {
        match self
        {
            AttributeType::F1 => vk::Format::R32_SFLOAT,
            AttributeType::F2 => vk::Format::R32G32_SFLOAT,
            AttributeType::F3 => vk::Format::R32G32B32_SFLOAT,
            AttributeType::F4 => vk::Format::R32G32B32A32_SFLOAT,
        }
    }

    pub(crate) fn size_in_bytes(&self) -> u32
    {
        match self
        {
            AttributeType::F1 => 4,
            AttributeType::F2 => 8,
            AttributeType::F3 => 12,
            AttributeType::F4 => 16,
        }
    }
}

#[derive(gru_vulkan_derive::AttributeType)]
#[repr(C, packed)]
pub struct F1(f32);

impl From<f32> for F1
{
    fn from(x: f32) -> Self
    {
        F1(x)
    }
}

impl Into<f32> for F1
{
    fn into(self) -> f32
    {
        self.0
    }
}

#[derive(gru_vulkan_derive::AttributeType)]
#[repr(C, packed)]
pub struct F2(f32, f32);

impl From<(f32, f32)> for F2
{
    fn from((x, y): (f32, f32)) -> Self
    {
        F2(x, y)
    }
}

impl Into<(f32, f32)> for F2
{
    fn into(self) -> (f32, f32)
    {
        (self.0, self.1)
    }
}

impl From<Vec2> for F2
{
    fn from(Vec2(x, y): Vec2) -> Self
    {
        F2(x, y)
    }
}

impl Into<Vec2> for F2
{
    fn into(self) -> Vec2
    {
        Vec2(self.0, self.1)
    }
}

#[derive(gru_vulkan_derive::AttributeType)]
#[repr(C, packed)]
pub struct F3(f32, f32, f32);

impl From<(f32, f32, f32)> for F3
{
    fn from((x, y, z): (f32, f32, f32)) -> Self
    {
        F3(x, y, z)
    }
}

impl Into<(f32, f32, f32)> for F3
{
    fn into(self) -> (f32, f32, f32)
    {
        (self.0, self.1, self.2)
    }
}

impl From<Vec3> for F3
{
    fn from(Vec3(x, y, z): Vec3) -> Self
    {
        F3(x, y, z)
    }
}

impl Into<Vec3> for F3
{
    fn into(self) -> Vec3
    {
        Vec3(self.0, self.1, self.2)
    }
}

#[derive(gru_vulkan_derive::AttributeType)]
#[repr(C, packed)]
pub struct F4(f32, f32, f32, f32);

impl From<(f32, f32, f32, f32)> for F4
{
    fn from((x, y, z, w): (f32, f32, f32, f32)) -> Self
    {
        F4(x, y, z, w)
    }
}

impl Into<(f32, f32, f32, f32)> for F4
{
    fn into(self) -> (f32, f32, f32, f32)
    {
        (self.0, self.1, self.2, self.3)
    }
}

impl From<Vec4> for F4
{
    fn from(Vec4(x, y, z, w): Vec4) -> Self
    {
        F4(x, y, z, w)
    }
}

impl Into<Vec4> for F4
{
    fn into(self) -> Vec4
    {
        Vec4(self.0, self.1, self.2, self.3)
    }
}

//     #####     BUFFER     #####

impl Device
{
    pub fn new_buffer_layout(&self) -> BufferLayout
    {
        let mut id = self.0.buffer_layout_count.lock().unwrap();
        *id += 1;
        let uniform_align = unsafe { self.0.instance.instance.get_physical_device_properties(self.0.physical_device).limits.min_uniform_buffer_offset_alignment };
        BufferLayout
        {
            id: *id,
            offset_in_bytes: 0,
            uniform_align,
            indices: false,
            attributes: false,
            uniforms: false,
            sealed: false
        }
    }

    pub fn new_buffer(&self, buffer_layout: &mut BufferLayout, memory_type: MemoryType, transfer_type: TransferType) -> Buffer
    {
        //if DEBUG_MODE && buffer_layout.offset_in_bytes == 0 { panic!("Device::new_buffer: No empty buffers allowed."); }
        buffer_layout.sealed = true;
        let memory_usage_flags = match memory_type
        {
            MemoryType::CpuToGpu => vk_mem::MemoryUsage::CpuToGpu,
            MemoryType::GpuToCpu => vk_mem::MemoryUsage::GpuToCpu,
            MemoryType::CpuOnly => vk_mem::MemoryUsage::CpuOnly,
            MemoryType::GpuOnly => vk_mem::MemoryUsage::GpuOnly
        };
        let allocation_create_info = vk_mem::AllocationCreateInfo { usage: memory_usage_flags, ..Default::default() };
        let buffer_usage_flags = match transfer_type
        {
            TransferType::None => vk::BufferUsageFlags::empty(),
            TransferType::Src => vk::BufferUsageFlags::TRANSFER_SRC,
            TransferType::Dst => vk::BufferUsageFlags::TRANSFER_DST,
            TransferType::Both => vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST
        }
        | if buffer_layout.indices { vk::BufferUsageFlags::INDEX_BUFFER } else { vk::BufferUsageFlags::empty() }
        | if buffer_layout.attributes { vk::BufferUsageFlags::VERTEX_BUFFER } else { vk::BufferUsageFlags::empty() }
        | if buffer_layout.uniforms { vk::BufferUsageFlags::UNIFORM_BUFFER } else { vk::BufferUsageFlags::empty() };
        let (buffer, allocation, allocation_info) = self.0.allocator.create_buffer
        (
            &ash::vk::BufferCreateInfo::builder()
                .size(buffer_layout.offset_in_bytes)
                .usage(buffer_usage_flags)
                .build(),
            &allocation_create_info,
        ).unwrap();
        Buffer
        {
            device: self.0.clone(),
            allocation,
            _allocation_info: allocation_info,
            buffer,
            memory_type,
            transfer_type,
            layout_id: buffer_layout.id,
            size_in_bytes: buffer_layout.offset_in_bytes
        }
    }
}

impl AttributeGroupInfo
{
    pub fn from<T: AttributeGroupReprCpacked>() -> Self
    {
        Self
        {
            rate: T::RATE,
            attributes: T::ATTRIBUTES
        }
    }
}

impl<T> BufferView<T>
{
    pub fn count(&self) -> u32
    {
        self.count
    }
}

impl BufferLayout
{
    pub fn add_indices<T: IndexType>(&mut self, count: usize) -> BufferView<T>
    {
        self.indices = true;
        self.add(count, std::mem::size_of::<T>() as u64)
    }

    pub fn add_attributes<T: AttributeGroupReprCpacked>(&mut self, count: usize) -> BufferView<T>
    {
        self.attributes = true;
        self.add(count, 1)
    }

    pub fn add_uniforms<T: DescriptorStructReprC>(&mut self, count: usize) -> BufferView<T>
    {
        self.uniforms = true;
        self.add(count, self.uniform_align)
    }

    fn add<T>(&mut self, count: usize, align: u64) -> BufferView<T>
    {
        if DEBUG_MODE && self.sealed { panic!("BufferLayout::add: Layout is sealed after buffer creation."); }
        //if count == 0 { panic!("BufferLayout::add: No empty data permitted."); }
        let offset_overflow = self.offset_in_bytes % align;
        self.offset_in_bytes += if offset_overflow == 0 { 0 } else { align - offset_overflow };
        let begin_offset_in_bytes = self.offset_in_bytes;
        let stride = (std::mem::size_of::<T>() as u64 / align + (if std::mem::size_of::<T>() as u64 % align == 0 { 0 } else { 1 })) * align;
        self.offset_in_bytes += count as u64 * stride;
        BufferView
        {
            layout_id: self.id,
            offset_in_bytes: begin_offset_in_bytes as usize,
            count: count as u32,
            stride: stride as u32,
            phantom: PhantomData
        }
    }
}

#[derive(Clone, Copy)]
pub enum MemoryType
{
    CpuOnly,
    GpuOnly,
    CpuToGpu,
    GpuToCpu
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransferType
{
    None,
    Src,
    Dst,
    Both
}

impl Buffer
{
    pub fn map(&mut self) -> BufferMap
    {
        if DEBUG_MODE { if let MemoryType::GpuOnly | MemoryType::GpuToCpu = self.memory_type { panic!("Buffer::map: GPU buffers cannot be mapped.") } }
        let buffer_ptr = self.device.allocator.map_memory(&self.allocation).unwrap();
        BufferMap
        {
            buffer: self,
            buffer_ptr
        }
    }
}

pub struct BufferMap<'a>
{
    buffer: &'a mut Buffer,
    buffer_ptr: *mut u8,
}

impl<'a> BufferMap<'a>
{
    pub fn write_indices<T: IndexType>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        self.check(view, offset, data.len());
        self.write(view, offset, data);
    }

    pub fn write_attributes<T: AttributeGroupReprCpacked>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        self.check(view, offset, data.len());
        self.write(view, offset, data);
    }

    pub fn write_uniforms<T: DescriptorStructReprC>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        self.check(view, offset, data.len());
        unsafe
        {
            let buffer_ptr = self.buffer_ptr.add(view.offset_in_bytes + offset * view.stride as usize);
            ash::util::Align::new(buffer_ptr as *mut std::ffi::c_void, view.stride as u64, (view.stride * data.len() as u32) as u64).copy_from_slice(data);
        }
    }

    #[inline(always)]
    fn check<T>(&self, view: &BufferView<T>, offset: usize, count: usize)
    {
        if DEBUG_MODE && view.layout_id != self.buffer.layout_id { panic!("BufferMap_write_?: Incompatible BufferView."); }
        if offset + count > view.count as usize { panic!("BufferMap_write_?: Too much data."); } //TODO only in DEBUG_MODE?
    }

    #[inline(always)]
    fn write<T>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        unsafe
        {
            let buffer_ptr = (self.buffer_ptr.add(view.offset_in_bytes) as *mut T).add(offset);
            buffer_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        }
    }
}

impl Drop for BufferMap<'_>
{
    fn drop(&mut self)
    {
        self.buffer.device.allocator.unmap_memory(&self.buffer.allocation).unwrap();
    }
}

impl<'a> CommandBuffer<'a>
{
    pub fn copy_buffer<'b, 'c>(self, queue: &Queue, src: &'b Buffer, dst: &'c Buffer, mark: Fence) -> CopyBufferFence<'a, 'b, 'c>
    {
        if DEBUG_MODE && self.pool.queue_family_index != queue.index { panic!("CommandBuffer::copy_buffer: Wrong queue family."); }
        if DEBUG_MODE && !self.pool.queue_family_flags.contains(vk::QueueFlags::TRANSFER) { panic!("CommandBuffer::copy_buffer: This queue family does not support transfer operations."); }
        if DEBUG_MODE && !(src.transfer_type == TransferType::Src) && !(src.transfer_type == TransferType::Both) { panic!("CommandBuffer::copy_buffer: Source buffer does not support transfer."); }
        if DEBUG_MODE && !(dst.transfer_type == TransferType::Dst) && !(dst.transfer_type == TransferType::Both) { panic!("CommandBuffer::copy_buffer: Destination buffer does not support transfer."); }
        if DEBUG_MODE && src.layout_id != dst.layout_id { panic!("CommandBuffer::copy_buffer: Buffer need to have the same layout."); }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let buffer_copy = vk::BufferCopy
        {
            src_offset: 0,
            dst_offset: 0,
            size: src.size_in_bytes
        };
        let submit_info =
        [   vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(&self.command_buffer))
            .build()
        ];
        unsafe
        {
            self.pool.device.logical_device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info).unwrap();
            self.pool.device.logical_device.cmd_copy_buffer(self.command_buffer, src.buffer, dst.buffer, &[buffer_copy]);
            self.pool.device.logical_device.end_command_buffer(self.command_buffer).unwrap();
            self.pool.device.logical_device.queue_submit(queue.queue, &submit_info, mark.fence).unwrap();
        }
        CopyBufferFence { mark, _command_buffer: self, _src: src, _dst: dst }
    }
}

pub struct CopyBufferFence<'a, 'b, 'c>
{
    pub mark: Fence,
    _command_buffer: CommandBuffer<'a>,
    _src: &'b Buffer,
    _dst: &'c Buffer
}

impl<'a> IndexBinding<'a>
{
    pub fn from<T: IndexType>(buffer: &'a Buffer, view: &BufferView<T>) -> Self
    {
        if DEBUG_MODE && buffer.layout_id != view.layout_id { panic!("IndexBinding::from: Incompatible BufferView."); }
        Self
        {
            buffer,
            offset_in_bytes: view.offset_in_bytes as u64,
            format: T::FORMAT
        }
    }
}

impl<'a> AttributeBinding<'a>
{
    pub fn from<T: AttributeGroupReprCpacked>(buffer: &'a Buffer, view: &BufferView<T>) -> Self
    {
        if DEBUG_MODE && buffer.layout_id != view.layout_id { panic!("AttributeBinding::from: Incompatible BufferView."); }
        Self
        {
            buffer,
            offset_in_bytes: view.offset_in_bytes as u64
        }
    }
}
