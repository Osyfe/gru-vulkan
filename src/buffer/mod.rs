mod data;
pub use data::*;

use super::*;

impl Device
{
    pub fn new_buffer_type(&self) -> BufferTypeBuilder
    {
        let id = self.0.buffer_layout_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let buffer_type = BufferType
        {
            id,
            offset_in_bytes: 0,
            uniform_align: self.0.props.min_uniform_buffer_offset_alignment,
            storage_align: self.0.props.min_storage_buffer_offset_alignment,
            indices: false,
            attributes: false,
            uniforms: false,
            storage: false
        };
        BufferTypeBuilder(buffer_type)
    }

    pub fn new_buffer(&self, buffer_type: &BufferType, buffer_usage: BufferUsage) -> Buffer
    {
        if DEBUG_MODE && buffer_type.offset_in_bytes == 0 { panic!("Device::new_buffer: No empty buffers allowed."); }
        let (location, mut buffer_usage_flags) = match buffer_usage
        {
            BufferUsage::Stage => (gpu_allocator::MemoryLocation::CpuToGpu, vk::BufferUsageFlags::TRANSFER_SRC),
            BufferUsage::Dynamic => (gpu_allocator::MemoryLocation::CpuToGpu, vk::BufferUsageFlags::empty()),
            BufferUsage::Static => (gpu_allocator::MemoryLocation::GpuOnly, vk::BufferUsageFlags::TRANSFER_DST)
        };
        if buffer_usage != BufferUsage::Stage
        {
            buffer_usage_flags |=
                if buffer_type.indices { vk::BufferUsageFlags::INDEX_BUFFER } else { vk::BufferUsageFlags::empty() }
            | if buffer_type.attributes { vk::BufferUsageFlags::VERTEX_BUFFER } else { vk::BufferUsageFlags::empty() }
            | if buffer_type.uniforms { vk::BufferUsageFlags::UNIFORM_BUFFER } else { vk::BufferUsageFlags::empty() }
            | if buffer_type.storage { vk::BufferUsageFlags::STORAGE_BUFFER } else { vk::BufferUsageFlags::empty() };
        }
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(buffer_type.offset_in_bytes)
            .usage(buffer_usage_flags);

        let device = &self.0.logical_device;
        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }.unwrap();
        let allocation_create_desc = alloc::AllocationCreateDesc
        {
            name: "",
            requirements: unsafe { device.get_buffer_memory_requirements(buffer) },
            location,
            linear: true,
            allocation_scheme: alloc::AllocationScheme::GpuAllocatorManaged
        };
        let allocation = self.0.allocator.as_ref().unwrap().lock().unwrap().allocate(&allocation_create_desc).unwrap();
        unsafe { device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()).unwrap(); }
        
        Buffer
        {
            device: self.0.clone(),
            allocation: Some(allocation),
            buffer,
            buffer_usage,
            layout_id: buffer_type.id,
            size_in_bytes: buffer_type.offset_in_bytes
        }
    }
}

impl AttributeGroupInfo
{
    pub const fn from<T: AttributeGroupReprCpacked>() -> Self
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
    pub const fn count(&self) -> u32
    {
        self.count
    }
}

impl BufferTypeBuilder
{
    pub fn add_indices<T: IndexType>(&mut self, count: u32) -> BufferView<T> { self.add_indices_internal(count, false) }
    pub fn add_attributes<T: AttributeGroupReprCpacked>(&mut self, count: u32) -> BufferView<T> { self.add_attributes_internal(count, false) }
    pub fn add_uniforms<T: DescriptorStructReprC>(&mut self, count: u32) -> BufferView<T> { self.add_uniforms_internal(count, false) }
    pub fn add_indices_storage<T: IndexType + StorageStructReprC>(&mut self, count: u32) -> BufferView<T> { self.add_indices_internal(count, true) }
    pub fn add_attributes_storage<T: AttributeGroupReprCpacked + StorageStructReprC>(&mut self, count: u32) -> BufferView<T> { self.add_attributes_internal(count, true) }
    pub fn add_uniforms_storage<T: DescriptorStructReprC + StorageStructReprC>(&mut self, count: u32) -> BufferView<T> { self.add_uniforms_internal(count, true) }
    pub fn add_storage<T: StorageStructReprC>(&mut self, count: u32) -> BufferView<T> { self.add(count, 1, 1, true) } //add does offset_alignment

    fn ggt(mut a: u64, mut b: u64) -> u64
    {
        if a == 0 { return b; }
        while b != 0
        {
            if a > b { a = a - b; }
            else { b = b - a; }
        }
        a
    }

    fn kgv(a: u64, b: u64) -> u64
    {
        a * b / Self::ggt(a, b)
    }
    
    fn add_indices_internal<T: IndexType>(&mut self, count: u32, storage: bool) -> BufferView<T>
    {
        self.0.indices = true;
        self.add(count, std::mem::size_of::<T>() as u64, 1, storage)
    }

    fn add_attributes_internal<T: AttributeGroupReprCpacked>(&mut self, count: u32, storage: bool) -> BufferView<T>
    {
        self.0.attributes = true;
        self.add(count, 1, 1, storage)
    }

    fn add_uniforms_internal<T: DescriptorStructReprC>(&mut self, count: u32, storage: bool) -> BufferView<T>
    {
        self.0.uniforms = true;
        self.add(count, self.0.uniform_align, self.0.uniform_align, storage) //stride_align because multiple uniforms -> uniform array with offset_align for everyone
    }

    fn add<T>(&mut self, count: u32, mut offset_align: u64, stride_align: u64, storage: bool) -> BufferView<T>
    {
        if storage
        {
            self.0.storage = true;
            offset_align = Self::kgv(offset_align, self.0.storage_align);
        }
        //if count == 0 { panic!("BufferLayout::add: No empty data permitted."); }
        let offset_overflow = self.0.offset_in_bytes % offset_align;
        self.0.offset_in_bytes += if offset_overflow == 0 { 0 } else { offset_align - offset_overflow };
        let begin_offset_in_bytes = self.0.offset_in_bytes;
        let stride = (std::mem::size_of::<T>() as u64 / stride_align + (if std::mem::size_of::<T>() as u64 % stride_align == 0 { 0 } else { 1 })) * stride_align;
        self.0.offset_in_bytes += count as u64 * stride;
        BufferView
        {
            layout_id: self.0.id,
            offset_in_bytes: begin_offset_in_bytes as usize,
            count,
            stride: stride as u32,
            phantom: PhantomData
        }
    }

    pub fn build(self) -> BufferType
    {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum BufferUsage
{
    Stage,
    Dynamic,
    Static
}
//gpu_allocator does persistent mapping, but we keep this BufferMap API around for a potential swap to gpu_alloc
impl Buffer
{
    #[inline]
    pub fn map(&mut self) -> BufferMap
    {
        if DEBUG_MODE { if self.buffer_usage == BufferUsage::Static { panic!("Buffer::map: Static buffers cannot be mapped.") } }
        let buffer_ptr = self.allocation.as_ref().unwrap().mapped_ptr().unwrap().as_ptr();
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
    buffer_ptr: *mut std::ffi::c_void,
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

    #[inline]
    pub fn write_storage<T: StorageStructReprC>(&mut self, view: &BufferView<T>, offset: usize, data: &[T])
    {
        self.check(view, offset, data.len());
        self.write(view, offset, data);
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

/* Unmap Buffer here if not persistent mapped.
impl Drop for BufferMap<'_>
{
    #[inline]
    fn drop(&mut self)
    {
    }
}
 */

impl CommandBuffer
{
    pub fn copy_buffer<'a, 'b>(self, queue: &Queue, src: &'a Buffer, dst: &'b Buffer, mark: Fence) -> CopyFence<'a, 'b>
    {
        if DEBUG_MODE && self.pool.queue_family_index != queue.index { panic!("CommandBuffer::copy_buffer: Wrong queue family."); }
        if DEBUG_MODE && !self.pool.queue_family_flags.contains(vk::QueueFlags::TRANSFER) { panic!("CommandBuffer::copy_buffer: This queue family does not support transfer operations."); }
        if DEBUG_MODE && src.buffer_usage != BufferUsage::Stage { panic!("CommandBuffer::copy_buffer: Source buffer has not stage memory type."); }
        if DEBUG_MODE && dst.buffer_usage != BufferUsage::Static { panic!("CommandBuffer::copy_buffer: Destination buffer has not static memory type."); }
        if DEBUG_MODE && src.layout_id != dst.layout_id { panic!("CommandBuffer::copy_buffer: Buffer need to have the same layout."); }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let buffer_copy = vk::BufferCopy
        {
            src_offset: 0,
            dst_offset: 0,
            size: src.size_in_bytes
        };
        let submit_info =
        [   vk::SubmitInfo::default()
            .command_buffers(std::slice::from_ref(&self.command_buffer))
        ];
        unsafe
        {
            self.pool.device.logical_device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info).unwrap();
            self.pool.device.logical_device.cmd_copy_buffer(self.command_buffer, src.buffer, dst.buffer, &[buffer_copy]);
            self.pool.device.logical_device.end_command_buffer(self.command_buffer).unwrap();
            self.pool.device.logical_device.queue_submit(queue.queue, &submit_info, mark.fence).unwrap();
        }
        CopyFence { mark, command_buffer: self, _src: &(), _dst: &() }
    }
}

impl<'a> CommandBufferRecord<'a>
{
    pub fn copy_view<T>(&self, src_buf: &Buffer, src_view: &BufferView<T>, dst_buf: &Buffer, dst_view: &BufferView<T>, usage: CopyViewUsage, stage: WaitStage)
    {
        if DEBUG_MODE && src_buf.buffer_usage != BufferUsage::Stage { panic!("CommandBuffer::copy_view: Source buffer has not stage memory type."); }
        if DEBUG_MODE && dst_buf.buffer_usage != BufferUsage::Static { panic!("CommandBuffer::copy_view: Destination buffer has not static memory type."); }
        if DEBUG_MODE && src_buf.layout_id != src_view.layout_id { panic!("CommandBuffer::copy_view: Source buffer and view are not compatible."); }
        if DEBUG_MODE && dst_buf.layout_id != dst_view.layout_id { panic!("CommandBuffer::copy_view: Destination buffer and view are not compatible."); }
        if DEBUG_MODE && src_view.count != dst_view.count { panic!("CommandBuffer::copy_view: Source and destination views have different counts."); }

        let size = std::mem::size_of::<T>() as u64 * src_view.count as u64;
        let buffer_copy = vk::BufferCopy
        {
            src_offset: src_view.offset_in_bytes as u64,
            dst_offset: dst_view.offset_in_bytes as u64,
            size
        };
        let memory_barrier = vk::BufferMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(usage.vk_access_flags())
            .src_queue_family_index(self.buffer.pool.queue_family_index as u32)
            .dst_queue_family_index(self.buffer.pool.queue_family_index as u32)
            .buffer(dst_buf.buffer)
            .offset(dst_view.offset_in_bytes as u64)
            .size(size);
        unsafe
        {
            self.buffer.pool.device.logical_device.cmd_copy_buffer(self.buffer.command_buffer, src_buf.buffer, dst_buf.buffer, &[buffer_copy]);
            self.buffer.pool.device.logical_device.cmd_pipeline_barrier(self.buffer.command_buffer, vk::PipelineStageFlags::TRANSFER, stage.vk_mask(), vk::DependencyFlags::empty(), &[], &[memory_barrier], &[]);
        }
    }
}

#[derive(Clone, Copy)]
pub enum CopyViewUsage
{
    Uniform
}

impl CopyViewUsage
{
    fn vk_access_flags(self) -> vk::AccessFlags
    {
        match self
        {
            Self::Uniform => vk::AccessFlags::UNIFORM_READ
        }
    }
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
