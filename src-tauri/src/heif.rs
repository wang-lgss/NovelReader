// HEIC 解码：随应用携带 libheif，避免依赖 Windows HEVC 扩展。
// 常量：heif_colorspace_RGB = 1，heif_chroma_interleaved_RGB = 10。
#[cfg(windows)]
use std::ffi::{c_char, c_int, c_void, CStr};
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::OnceLock;

#[cfg(windows)]
#[repr(C)]
struct HeifError {
    code: c_int,
    subcode: c_int,
    message: *const c_char,
}

#[cfg(windows)]
type ContextAlloc = unsafe extern "C" fn() -> *mut c_void;
#[cfg(windows)]
type ContextFree = unsafe extern "C" fn(*mut c_void);
#[cfg(windows)]
type ReadFromMemory =
    unsafe extern "C" fn(*mut c_void, *const c_void, usize, *const c_void) -> HeifError;
#[cfg(windows)]
type GetPrimaryHandle = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> HeifError;
#[cfg(windows)]
type HandleGetWidth = unsafe extern "C" fn(*mut c_void) -> c_int;
#[cfg(windows)]
type HandleGetHeight = unsafe extern "C" fn(*mut c_void) -> c_int;
#[cfg(windows)]
type DecodeImage =
    unsafe extern "C" fn(*mut c_void, *mut *mut c_void, c_int, c_int, *const c_void) -> HeifError;
#[cfg(windows)]
type ImageGetWidth = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
#[cfg(windows)]
type ImageGetHeight = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
#[cfg(windows)]
type ImageGetPlane = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> *const u8;
#[cfg(windows)]
type ImageRelease = unsafe extern "C" fn(*mut c_void);
#[cfg(windows)]
type HandleRelease = unsafe extern "C" fn(*mut c_void);

#[cfg(windows)]
struct HeifApi {
    context_alloc: ContextAlloc,
    context_free: ContextFree,
    read_from_memory: ReadFromMemory,
    get_primary: GetPrimaryHandle,
    handle_width: HandleGetWidth,
    handle_height: HandleGetHeight,
    decode_image: DecodeImage,
    image_width: ImageGetWidth,
    image_height: ImageGetHeight,
    image_plane: ImageGetPlane,
    image_release: ImageRelease,
    handle_release: HandleRelease,
}

#[cfg(windows)]
static HEIF_API: OnceLock<Result<HeifApi, String>> = OnceLock::new();

#[cfg(windows)]
fn find_heif_dir() -> Option<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("resources").join("heif"));
            dirs.push(parent.join("heif"));
            if let Some(grand) = parent.parent() {
                dirs.push(grand.join("resources").join("heif"));
            }
        }
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("heif"));
    dirs.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("src-tauri")
            .join("resources")
            .join("heif"),
    );
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("resources").join("heif"));
    }
    dirs.into_iter().find(|p| p.is_dir())
}

#[cfg(windows)]
fn load_api() -> Result<HeifApi, String> {
    let dir = find_heif_dir().ok_or("找不到 libheif 资源目录")?;
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("读取 libheif 目录失败: {}", e))?;
    let mut dll = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("libheif-") && name.ends_with(".dll") {
            dll = Some(entry.path());
            break;
        }
    }
    let dll = dll.ok_or_else(|| format!("{} 中没有 libheif DLL", dir.display()))?;
    let lib = unsafe {
        libloading::os::windows::Library::load_with_flags(
            &dll,
            0x00000008, // LOAD_WITH_ALTERED_SEARCH_PATH
        )
    }
    .map_err(|e| format!("加载 libheif 失败: {}", e))?;

    unsafe {
        let get = |name: &[u8]| -> Result<*mut c_void, String> {
            let sym = lib
                .get::<*mut c_void>(name)
                .map_err(|e| format!("libheif 缺少 {}: {}", String::from_utf8_lossy(&name[..name.len() - 1]), e))?;
            Ok(*sym)
        };
        let api = HeifApi {
            context_alloc: std::mem::transmute(get(b"heif_context_alloc\0")?),
            context_free: std::mem::transmute(get(b"heif_context_free\0")?),
            read_from_memory: std::mem::transmute(get(
                b"heif_context_read_from_memory_without_copy\0",
            )?),
            get_primary: std::mem::transmute(get(b"heif_context_get_primary_image_handle\0")?),
            handle_width: std::mem::transmute(get(b"heif_image_handle_get_width\0")?),
            handle_height: std::mem::transmute(get(b"heif_image_handle_get_height\0")?),
            decode_image: std::mem::transmute(get(b"heif_decode_image\0")?),
            image_width: std::mem::transmute(get(b"heif_image_get_width\0")?),
            image_height: std::mem::transmute(get(b"heif_image_get_height\0")?),
            image_plane: std::mem::transmute(get(b"heif_image_get_plane_readonly\0")?),
            image_release: std::mem::transmute(get(b"heif_image_release\0")?),
            handle_release: std::mem::transmute(get(b"heif_image_handle_release\0")?),
        };
        std::mem::forget(lib);
        Ok(api)
    }
}

#[cfg(windows)]
fn get_api() -> Result<&'static HeifApi, String> {
    HEIF_API
        .get_or_init(load_api)
        .as_ref()
        .map_err(|e| e.clone())
}

#[cfg(windows)]
fn check_error(err: HeifError, action: &str) -> Result<(), String> {
    if err.code == 0 {
        return Ok(());
    }
    let message = if err.message.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(err.message) }
            .to_string_lossy()
            .into_owned()
    };
    Err(format!("libheif {}失败: code={} subcode={} {}", action, err.code, err.subcode, message))
}

#[cfg(windows)]
pub fn decode_heic(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 12 {
        return Err("HEIC 数据为空".into());
    }
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("HEIC 图片过大".into());
    }
    let api = get_api()?;
    unsafe {
        let ctx = (api.context_alloc)();
        if ctx.is_null() {
            return Err("libheif 创建 context 失败".into());
        }
        let mut handle: *mut c_void = std::ptr::null_mut();
        let mut image: *mut c_void = std::ptr::null_mut();
        let result = (|| {
            let err = (api.read_from_memory)(
                ctx,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
                std::ptr::null(),
            );
            check_error(err, "读取 HEIC")?;
            let err = (api.get_primary)(ctx, &mut handle);
            check_error(err, "读取主图")?;
            if handle.is_null() {
                return Err("HEIC 没有主图".into());
            }
            let width = (api.handle_width)(handle);
            let height = (api.handle_height)(handle);
            if width <= 0 || height <= 0 || width > 4096 || height > 4096 {
                return Err(format!("HEIC 尺寸异常: {}x{}", width, height));
            }
            let err = (api.decode_image)(handle, &mut image, 1, 10, std::ptr::null());
            check_error(err, "解码 RGB")?;
            if image.is_null() {
                return Err("libheif 返回空图像".into());
            }
            let w = (api.image_width)(image, 10);
            let h = (api.image_height)(image, 10);
            if w <= 0 || h <= 0 {
                return Err("libheif 图像尺寸异常".into());
            }
            let mut stride: c_int = 0;
            let plane = (api.image_plane)(image, 10, &mut stride);
            if plane.is_null() || stride < w * 3 {
                return Err("libheif RGB 数据不可用".into());
            }
            let stride = stride as usize;
            let w = w as usize;
            let h = h as usize;
            let mut rgb = vec![0u8; w * h * 3];
            for row in 0..h {
                let src = plane.add(row * stride);
                let dst = rgb[row * w * 3..(row + 1) * w * 3].as_mut_ptr();
                std::ptr::copy_nonoverlapping(src, dst, w * 3);
            }
            let mut jpeg = Vec::new();
            let encoder = jpeg_encoder::Encoder::new(&mut jpeg, 82);
            encoder
                .encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb)
                .map_err(|e| format!("JPEG 编码失败: {}", e))?;
            Ok(jpeg)
        })();
        if !image.is_null() {
            (api.image_release)(image);
        }
        if !handle.is_null() {
            (api.handle_release)(handle);
        }
        (api.context_free)(ctx);
        result
    }
}

#[cfg(not(windows))]
pub fn decode_heic(_bytes: &[u8]) -> Result<Vec<u8>, String> {
    Err("HEIC 解码目前仅支持 Windows".into())
}
