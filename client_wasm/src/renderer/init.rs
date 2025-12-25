use web_sys::HtmlCanvasElement;
use wgpu::*;

pub struct WgpuContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub size: (u32, u32),
}

pub async fn init_wgpu(canvas: HtmlCanvasElement) -> Result<WgpuContext, String> {
    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let canvas_clone = canvas.clone();
    let surface = instance
        .create_surface(SurfaceTarget::Canvas(canvas_clone))
        .map_err(|e| format!("Failed to create surface: {:?}", e))?;

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| "Failed to find adapter".to_string())?;

    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: Some("Device"),
                required_features: Features::empty(),
                required_limits: Limits::downlevel_webgl2_defaults(),
                memory_hints: MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| format!("Failed to create device: {:?}", e))?;

    let width = canvas.width();
    let height = canvas.height();
    let size = (width, height);

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or_else(|| {
            surface_caps
                .formats
                .first()
                .copied()
                .expect("No surface formats available")
        });

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: PresentMode::Fifo,
        alpha_mode: CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    Ok(WgpuContext {
        device,
        queue,
        surface,
        config,
        size,
    })
}
