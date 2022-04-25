# Benchmarco

![APIs](https://img.shields.io/badge/Rust-gray?logo=rust&style=flat-square)
![APIs](https://img.shields.io/badge/Vulkan-gray?logo=Vulkan&style=flat-square)
![Platforms](https://img.shields.io/badge/platforms-windows%20%7C%20linux%20-red?style=flat-square)

Benchmarco is a simple graphical user interface to analize CPU, GPU and memory performance.

![Screenshot](./screenshots/1.png)

### Understanding clock values
- gpc
    - frequency of graphics (shader) clock.
- sm
    - frequency of SM (Streaming Multiprocessor) clock.
- mem
    - frequency of memory clock.
- vdo
    - frequency of video encoder/decoder clock.


## 🚀 Running
```
cargo run --release
```

## License
Use the code as you want :)