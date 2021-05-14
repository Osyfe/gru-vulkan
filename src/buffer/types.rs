use super::*;

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
    #[inline]
    fn from(x: f32) -> Self
    {
        F1(x)
    }
}

impl Into<f32> for F1
{
    #[inline]
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
    #[inline]
    fn from((x, y): (f32, f32)) -> Self
    {
        F2(x, y)
    }
}

impl Into<(f32, f32)> for F2
{
    #[inline]
    fn into(self) -> (f32, f32)
    {
        (self.0, self.1)
    }
}

impl From<Vec2> for F2
{
    #[inline]
    fn from(Vec2(x, y): Vec2) -> Self
    {
        F2(x, y)
    }
}

impl Into<Vec2> for F2
{
    #[inline]
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
    #[inline]
    fn from((x, y, z): (f32, f32, f32)) -> Self
    {
        F3(x, y, z)
    }
}

impl Into<(f32, f32, f32)> for F3
{
    #[inline]
    fn into(self) -> (f32, f32, f32)
    {
        (self.0, self.1, self.2)
    }
}

impl From<Vec3> for F3
{
    #[inline]
    fn from(Vec3(x, y, z): Vec3) -> Self
    {
        F3(x, y, z)
    }
}

impl Into<Vec3> for F3
{
    #[inline]
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
    #[inline]
    fn from((x, y, z, w): (f32, f32, f32, f32)) -> Self
    {
        F4(x, y, z, w)
    }
}

impl Into<(f32, f32, f32, f32)> for F4
{
    #[inline]
    fn into(self) -> (f32, f32, f32, f32)
    {
        (self.0, self.1, self.2, self.3)
    }
}

impl From<Vec4> for F4
{
    #[inline]
    fn from(Vec4(x, y, z, w): Vec4) -> Self
    {
        F4(x, y, z, w)
    }
}

impl Into<Vec4> for F4
{
    #[inline]
    fn into(self) -> Vec4
    {
        Vec4(self.0, self.1, self.2, self.3)
    }
}
