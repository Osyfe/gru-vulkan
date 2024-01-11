A convenience wrapper around Vulkan written in Rust. Its features are:

- reduced boilerplate, including some default settings and behaviour (e.g. mipmap generation or attribute binding management)
- integrated "gpu-allocator" for memory management
- Drop implementations for all Vulkan objects. Lifetimes on the GPU side (i.e. which objects are currently in use) are not tracked, this has to be done by the user.
- additional type safety in some places, especially around buffer management
