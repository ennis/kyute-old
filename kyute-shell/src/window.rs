//! Platform-specific window creation
use crate::{
    bindings::Windows::Win32::{
        Direct2D::{
            D2D1_ALPHA_MODE, D2D1_BITMAP_OPTIONS, D2D1_BITMAP_PROPERTIES1,
            D2D1_DEVICE_CONTEXT_OPTIONS, D2D1_PIXEL_FORMAT,
        },
        Dxgi::{
            IDXGISurface, IDXGISwapChain1, DXGI_ALPHA_MODE, DXGI_FORMAT, DXGI_SAMPLE_DESC,
            DXGI_SCALING, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT, DXGI_USAGE_RENDER_TARGET_OUTPUT,
        },
        SystemServices::HINSTANCE,
        WindowsAndMessaging::HWND,
    },
    drawing::{DrawContext, PhysicalSize},
    error::Error,
    platform::Platform,
};
use std::{
    ops::{Deref, DerefMut},
    ptr,
    sync::MutexGuard,
};
use windows::Interface;
use winit::{
    event_loop::EventLoopWindowTarget,
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Window, WindowBuilder, WindowId},
};

const SWAP_CHAIN_BUFFERS: u32 = 2;

/// Context object to draw on a window.
///
/// It implicitly derefs to [`DrawContext`], which has methods to draw primitives on the
/// window surface.
///
/// [`DrawContext`]: crate::drawing::context::DrawContext
pub struct WindowDrawContext<'a> {
    window: &'a mut PlatformWindow,
    draw_context: DrawContext,
}

impl<'a> WindowDrawContext<'a> {
    /// Creates a new [`WindowDrawContext`] for the specified window, allowing to draw on the window.
    pub fn new(window: &'a mut PlatformWindow) -> WindowDrawContext<'a> {
        let platform = Platform::instance();
        let d2d_device_context = &platform.0.d2d_device_context;

        let swap_chain = &window.swap_chain;
        let backbuffer = unsafe { swap_chain.GetBuffer::<IDXGISurface>(0).unwrap() };
        let dpi = 96.0 * window.window.scale_factor() as f32;

        // create target bitmap
        let mut bitmap = unsafe {
            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT::DXGI_FORMAT_R8G8B8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE::D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: dpi,
                dpiY: dpi,
                bitmapOptions: D2D1_BITMAP_OPTIONS::D2D1_BITMAP_OPTIONS_TARGET
                    | D2D1_BITMAP_OPTIONS::D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: None,
            };
            let mut bitmap = None;
            d2d_device_context
                .CreateBitmapFromDxgiSurface(backbuffer, &props, &mut bitmap)
                .and_some(bitmap)
                .expect("CreateBitmapFromDxgiSurface failed")
        };

        // create draw context
        let draw_context = unsafe {
            // set the target on the DC
            d2d_device_context.SetTarget(bitmap);
            d2d_device_context.SetDpi(dpi, dpi);
            // the draw context acquires shared ownership of the device context, but that's OK since we borrow the window,
            // so we can't create another WindowDrawContext that would conflict with it.
            DrawContext::from_device_context(platform.0.d2d_factory.0.clone(), d2d_device_context.0.clone())
        };

        WindowDrawContext {
            window,
            draw_context,
        }
    }

    /// Returns the [`PlatformWindow`] that is being drawn to.
    pub fn window(&self) -> &PlatformWindow {
        self.window
    }
}

impl<'a> Drop for WindowDrawContext<'a> {
    fn drop(&mut self) {
        // set the target to null to release the borrow of the backbuffer surface
        // (otherwise it will fail to resize)
        unsafe {
            self.ctx.SetTarget(None);
        }
    }
}

impl<'a> Deref for WindowDrawContext<'a> {
    type Target = DrawContext;
    fn deref(&self) -> &DrawContext {
        &self.draw_context
    }
}

impl<'a> DerefMut for WindowDrawContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.draw_context
    }
}

/// Encapsulates a Win32 window and associated resources for drawing to it.
pub struct PlatformWindow {
    window: Window,
    hwnd: HWND,
    hinstance: HINSTANCE,
    swap_chain: IDXGISwapChain1,
}

impl PlatformWindow {
    /// Returns the underlying winit [`Window`].
    ///
    /// [`Window`]: winit::Window
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Returns the underlying winit [`WindowId`].
    /// Equivalent to calling `self.window().id()`.
    ///
    /// [`WindowId`]: winit::WindowId
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    // Returns the rendering context associated to this window.
    //pub fn gpu_context(&self) -> &GpuContext {
    //    Platform::instance().gpu_context()
    //}

    /// Resizes the swap chain and associated resources of the window.
    ///
    /// Must be called whenever winit sends a resize message.
    pub fn resize(&mut self, size: PhysicalSize) {
        //trace!("resizing swap chain: {}x{}", width, height);

        // resizing to 0x0 will fail, so don't bother
        if size.is_empty() {
            return;
        }

        let size_i = size.to_u32();

        unsafe {
            // resize the swap chain
            if let Err(err) = self
                .swap_chain
                .ResizeBuffers(
                    0,
                    size_i.width,
                    size_i.height,
                    DXGI_FORMAT::DXGI_FORMAT_UNKNOWN,
                    0,
                )
                .ok()
            {
                // it fails sometimes, just log it
                tracing::error!("IDXGISwapChain1::ResizeBuffers failed: {}", err);
            }
        }
    }

    /// Creates a new window from the options given in the provided [`WindowBuilder`].
    ///
    /// To create the window with an OpenGL context, `with_gl` should be `true`.
    ///
    /// [`WindowBuilder`]: winit::WindowBuilder
    pub fn new(
        event_loop: &EventLoopWindowTarget<()>,
        mut builder: WindowBuilder,
        parent_window: Option<&PlatformWindow>,
    ) -> Result<PlatformWindow, Error> {
        let platform = Platform::instance();

        if let Some(parent_window) = parent_window {
            builder = builder.with_parent_window(parent_window.hwnd.0 as *mut _);
        }
        let window = builder.build(event_loop).map_err(Error::Winit)?;

        let dxgi_factory = &platform.0.dxgi_factory;
        let d3d11_device = &platform.0.d3d11_device;

        // create a DXGI swap chain for the window
        let hinstance = HINSTANCE(window.hinstance() as isize);
        let hwnd = HWND(window.hwnd() as isize);
        let (width, height): (u32, u32) = window.inner_size().into();

        // TODO flip effects
        let swap_effect = DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_SEQUENTIAL;

        // create the swap chain
        let swap_chain = unsafe {
            let mut swap_chain = None;

            let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT::DXGI_FORMAT_R8G8B8A8_UNORM,
                Stereo: false.into(),
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: SWAP_CHAIN_BUFFERS,
                Scaling: DXGI_SCALING::DXGI_SCALING_STRETCH,
                SwapEffect: swap_effect,
                AlphaMode: DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_UNSPECIFIED,
                Flags: 0,
            };

            dxgi_factory
                .CreateSwapChainForHwnd(
                    d3d11_device.0.clone(),
                    hwnd,
                    &swap_chain_desc,
                    ptr::null(),
                    None,
                    &mut swap_chain,
                )
                .and_some(swap_chain)
                .expect("failed to create swap chain")
        };

        let hinstance = HINSTANCE(window.hinstance() as isize);
        let hwnd = HWND(window.hwnd() as isize);

        let pw = PlatformWindow {
            window,
            hwnd,
            hinstance,
            swap_chain,
        };

        Ok(pw)
    }

    pub fn present(&mut self) {
        unsafe {
            if let Err(err) = self.swap_chain.Present(1, 0).ok() {
                tracing::error!("IDXGISwapChain::Present failed: {}", err)
            }
        }
    }
}
