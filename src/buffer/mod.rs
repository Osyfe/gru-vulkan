use super::*;
use gru_math::{Vec2, Vec3, Vec4};

mod types;
pub use types::*;

impl Device
{
    pub fn new_buffer_type(&self) -> BufferType
    {
        let mut id = self.0.buffer_layout_count.lock().unwrap();
        *id += 1;
        let uniform_align = unsafe { self.0.instance.instance.get_physical_device_properties(self.0.physical_device).limits.min_uniform_buffer_offset_alignment };
        BufferType
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

    pub fn new_buffer(&self, buffer_type: &mut BufferType, buffer_usage: BufferUsage) -> Buffer
    {
        //if DEBUG_MODE && buffer_layout.offset_in_bytes == 0 { panic!("Device::new_buffer: No empty buffers allowed."); }
        buffer_type.sealed = true;
        let (memory_usage_flags, mut buffer_usage_flags) = match buffer_usage
        {
            BufferUsage::Stage => (vk_mem::MemoryUsage::CpuOnly, vk::BufferUsageFlags::TRANSFER_SRC),
            BufferUsage::Dynamic => (vk_mem::MemoryUsage::CpuToGpu, vk::BufferUsageFlags::empty()),
            BufferUsage::Static => (vk_mem::MemoryUsage::GpuOnly, vk::BufferUsageFlags::TRANSFER_DST)
        };
        buffer_usage_flags |=
            if buffer_type.indices { vk::BufferUsageFlags::INDEX_BUFFER } else { vk::BufferUsageFlags::empty() }
          | if buffer_type.attributes { vk::BufferUsageFlags::VERTEX_BUFFER } else { vk::BufferUsageFlags::empty() }
          | if buffer_type.uniforms { vk::BufferUsageFlags::UNIFORM_BUFFER } else { vk::BufferUsageFlags::empty() };
        let allocation_create_info = vk_mem::AllocationCreateInfo { usage: memory_usage_flags, ..Default::default() };
        
        let (buffer, allocation, allocation_info) = self.0.allocator.create_buffer
        (
            &ash::vk::BufferCreateInfo::builder()
                .size(buffer_type.offset_in_bytes)
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
            buffer_usage,
            layout_id: buffer_type.id,
            size_in_bytes: buffer_type.offset_in_bytes
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
    #[inline]
    pub fn count(&self) -> u32
    {
        self.count
    }
}

impl BufferType
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
        if DEBUG_MODE && self.sealed { panic!("BufferLayout::add: Type is sealed after buffer creation."); }
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

#[derive(Clone, Copy, PartialEq)]
pub enum BufferUsage
{
    Stage,
    Dynamic,
    Static
}

impl Buffer
{
    #[inline]
    pub fn map(&mut self) -> BufferMap
    {
        if DEBUG_MODE { if self.buffer_usage == BufferUsage::Static { panic!("Buffer::map: Static buffers cannot be mapped.") } }
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
    #[inline]
    pub fn write_indices<T: IndexType>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        self.check(view, offset, data.len());
        self.write(view, offset, data);
    }

    #[inline]
    pub fn write_attributes<T: AttributeGroupReprCpacked>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        self.check(view, offset, data.len());
        self.write(view, offset, data);
    }

    #[inline]
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
    #[inline]
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
        if DEBUG_MODE && src.buffer_usage != BufferUsage::Stage { panic!("CommandBuffer::copy_buffer: Source buffer has not stage memory type."); }
        if DEBUG_MODE && dst.buffer_usage != BufferUsage::Static { panic!("CommandBuffer::copy_buffer: Destination buffer has not static memory type."); }
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
    #[inline]
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
    #[inline]
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