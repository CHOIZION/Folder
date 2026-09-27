#[cfg(target_os = "windows")]
mod windows_desktop {
    use jpeg_encoder::{ColorType, Encoder};
    use std::{ffi::c_void, mem::size_of};

    type Handle = *mut c_void;
    type Hdc = Handle;
    type Hbitmap = Handle;
    type Hgdiobj = Handle;

    #[repr(C)]
    struct RgbQuad {
        rgb_blue: u8,
        rgb_green: u8,
        rgb_red: u8,
        rgb_reserved: u8,
    }

    #[repr(C)]
    struct BitmapInfoHeader {
        bi_size: u32,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: u32,
        bi_size_image: u32,
        bi_x_pels_per_meter: i32,
        bi_y_pels_per_meter: i32,
        bi_clr_used: u32,
        bi_clr_important: u32,
    }

    #[repr(C)]
    struct BitmapInfo {
        bmi_header: BitmapInfoHeader,
        bmi_colors: [RgbQuad; 1],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MouseInput {
        dx: i32,
        dy: i32,
        mouse_data: u32,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct KeyboardInput {
        virtual_key: u16,
        scan: u16,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct HardwareInput {
        message: u32,
        low: u16,
        high: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    union InputData {
        mouse: MouseInput,
        keyboard: KeyboardInput,
        hardware: HardwareInput,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Input {
        input_type: u32,
        data: InputData,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetDC(window: Handle) -> Hdc;
        fn ReleaseDC(window: Handle, dc: Hdc) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
        fn SetCursorPos(x: i32, y: i32) -> i32;
        fn mouse_event(flags: u32, dx: i32, dy: i32, data: i32, extra_info: usize);
        fn keybd_event(key: u8, scan: u8, flags: u32, extra_info: usize);
        fn SendInput(count: u32, inputs: *const Input, input_size: i32) -> u32;
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Handle;
        fn ShowWindow(window: Handle, command: i32) -> i32;
        fn SetForegroundWindow(window: Handle) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleBitmap(dc: Hdc, width: i32, height: i32) -> Hbitmap;
        fn CreateCompatibleDC(dc: Hdc) -> Hdc;
        fn SetStretchBltMode(dc: Hdc, mode: i32) -> i32;
        fn StretchBlt(
            destination: Hdc,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            source: Hdc,
            source_x: i32,
            source_y: i32,
            source_width: i32,
            source_height: i32,
            operation: u32,
        ) -> i32;
        fn DeleteDC(dc: Hdc) -> i32;
        fn DeleteObject(object: Hgdiobj) -> i32;
        fn SelectObject(dc: Hdc, object: Hgdiobj) -> Hgdiobj;
        fn GetDIBits(
            dc: Hdc,
            bitmap: Hbitmap,
            start: u32,
            lines: u32,
            bits: *mut c_void,
            info: *mut BitmapInfo,
            usage: u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLastError() -> u32;
    }

    const SM_XVIRTUALSCREEN: i32 = 76;
    const SM_YVIRTUALSCREEN: i32 = 77;
    const SM_CXVIRTUALSCREEN: i32 = 78;
    const SM_CYVIRTUALSCREEN: i32 = 79;
    const BI_RGB: u32 = 0;
    const DIB_RGB_COLORS: u32 = 0;
    const SRCCOPY: u32 = 0x00CC_0020;
    const COLORONCOLOR: i32 = 3;
    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const KEYEVENTF_UNICODE: u32 = 0x0004;
    const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
    const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
    const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
    const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
    const MOUSEEVENTF_WHEEL: u32 = 0x0800;
    const MOUSEEVENTF_HWHEEL: u32 = 0x1000;
    const VK_BACK: u16 = 8;
    const VK_TAB: u16 = 9;
    const VK_RETURN: u16 = 13;
    const VK_ESCAPE: u16 = 27;
    const VK_SPACE: u16 = 32;
    const VK_PRIOR: u16 = 33;
    const VK_NEXT: u16 = 34;
    const VK_END: u16 = 35;
    const VK_HOME: u16 = 36;
    const VK_LEFT: u16 = 37;
    const VK_UP: u16 = 38;
    const VK_RIGHT: u16 = 39;
    const VK_DOWN: u16 = 40;
    const VK_DELETE: u16 = 46;
    const VK_F1: u16 = 112;
    const VK_F2: u16 = 113;
    const VK_F3: u16 = 114;
    const VK_F4: u16 = 115;
    const VK_F5: u16 = 116;
    const VK_F6: u16 = 117;
    const VK_F7: u16 = 118;
    const VK_F8: u16 = 119;
    const VK_F9: u16 = 120;
    const VK_F10: u16 = 121;
    const VK_F11: u16 = 122;
    const VK_F12: u16 = 123;
    const SW_RESTORE: i32 = 9;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DesktopGeometry {
        pub x: i32,
        pub y: i32,
        pub width: i32,
        pub height: i32,
    }

    pub fn geometry() -> Result<DesktopGeometry, String> {
        let result = unsafe {
            DesktopGeometry {
                x: GetSystemMetrics(SM_XVIRTUALSCREEN),
                y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
            }
        };
        if result.width <= 0 || result.height <= 0 {
            Err("Windows 화면 크기를 확인하지 못했습니다.".to_string())
        } else {
            Ok(result)
        }
    }

    /// Reuses GDI objects and pixel buffers while a screen stream is open.
    pub struct ScreenCapture {
        desktop_dc: Hdc,
        memory_dc: Hdc,
        bitmap: Hbitmap,
        previous: Hgdiobj,
        source: DesktopGeometry,
        target_width: i32,
        target_height: i32,
        bitmap_info: BitmapInfo,
        bgra: Vec<u8>,
        previous_bgra: Vec<u8>,
        nv12: Vec<u8>,
        jpeg: Vec<u8>,
    }

    impl ScreenCapture {
        pub fn new(max_width: i32) -> Result<Self, String> {
            Self::from_geometry(geometry()?, max_width)
        }

        fn from_geometry(source: DesktopGeometry, max_width: i32) -> Result<Self, String> {
            let (target_width, target_height) = scaled_dimensions(source, max_width);
            unsafe {
                let desktop_window = std::ptr::null_mut();
                let desktop_dc = GetDC(desktop_window);
                if desktop_dc.is_null() {
                    return Err("Windows 화면 DC를 열지 못했습니다.".to_string());
                }

                let memory_dc = CreateCompatibleDC(desktop_dc);
                if memory_dc.is_null() {
                    ReleaseDC(desktop_window, desktop_dc);
                    return Err("화면 캡처 DC를 만들지 못했습니다.".to_string());
                }

                let bitmap = CreateCompatibleBitmap(desktop_dc, target_width, target_height);
                if bitmap.is_null() {
                    DeleteDC(memory_dc);
                    ReleaseDC(desktop_window, desktop_dc);
                    return Err("화면 캡처 비트맵을 만들지 못했습니다.".to_string());
                }

                let previous = SelectObject(memory_dc, bitmap);
                if previous.is_null() || previous as isize == -1 {
                    let error_code = GetLastError();
                    DeleteObject(bitmap);
                    DeleteDC(memory_dc);
                    ReleaseDC(desktop_window, desktop_dc);
                    return Err(format!(
                        "화면 캡처 비트맵을 DC에 연결하지 못했습니다. (오류 코드 {error_code})"
                    ));
                }
                if SetStretchBltMode(memory_dc, COLORONCOLOR) == 0 {
                    let error_code = GetLastError();
                    SelectObject(memory_dc, previous);
                    DeleteObject(bitmap);
                    DeleteDC(memory_dc);
                    ReleaseDC(desktop_window, desktop_dc);
                    return Err(format!(
                        "Windows 화면 축소 모드를 설정하지 못했습니다. (오류 코드 {error_code})"
                    ));
                }

                let pixel_bytes = target_width as usize * target_height as usize * 4;
                Ok(Self {
                    desktop_dc,
                    memory_dc,
                    bitmap,
                    previous,
                    source,
                    target_width,
                    target_height,
                    bitmap_info: BitmapInfo {
                        bmi_header: BitmapInfoHeader {
                            bi_size: size_of::<BitmapInfoHeader>() as u32,
                            bi_width: target_width,
                            bi_height: -target_height,
                            bi_planes: 1,
                            bi_bit_count: 32,
                            bi_compression: BI_RGB,
                            bi_size_image: 0,
                            bi_x_pels_per_meter: 0,
                            bi_y_pels_per_meter: 0,
                            bi_clr_used: 0,
                            bi_clr_important: 0,
                        },
                        bmi_colors: [RgbQuad {
                            rgb_blue: 0,
                            rgb_green: 0,
                            rgb_red: 0,
                            rgb_reserved: 0,
                        }],
                    },
                    bgra: vec![0_u8; pixel_bytes],
                    previous_bgra: Vec::with_capacity(pixel_bytes),
                    nv12: Vec::with_capacity(pixel_bytes * 3 / 8),
                    jpeg: Vec::with_capacity(pixel_bytes / 8),
                })
            }
        }

        pub fn capture_frame(&mut self, max_width: i32) -> Result<(), String> {
            let source = geometry()?;
            let (target_width, target_height) = scaled_dimensions(source, max_width);
            if target_width != self.target_width || target_height != self.target_height {
                *self = Self::from_geometry(source, max_width)?;
            } else {
                self.source = source;
            }

            unsafe {
                let copied = StretchBlt(
                    self.memory_dc,
                    0,
                    0,
                    self.target_width,
                    self.target_height,
                    self.desktop_dc,
                    self.source.x,
                    self.source.y,
                    self.source.width,
                    self.source.height,
                    SRCCOPY,
                );
                if copied == 0 {
                    let error_code = GetLastError();
                    return Err(format!(
                        "Windows 화면 축소 복사에 실패했습니다. ({}x{} -> {}x{}, 위치 {},{}, 오류 코드 {})",
                        self.source.width,
                        self.source.height,
                        self.target_width,
                        self.target_height,
                        self.source.x,
                        self.source.y,
                        error_code
                    ));
                }

                let rows = GetDIBits(
                    self.memory_dc,
                    self.bitmap,
                    0,
                    self.target_height as u32,
                    self.bgra.as_mut_ptr() as *mut c_void,
                    &mut self.bitmap_info,
                    DIB_RGB_COLORS,
                );
                if rows == 0 {
                    return Err("Windows 화면 픽셀을 읽지 못했습니다.".to_string());
                }
            }

            Ok(())
        }

        /// Returns whether the captured frame differs from the previous one.
        /// Slice equality uses the platform's optimized memory comparison and
        /// avoids the per-byte multiply previously performed by FNV hashing.
        pub fn frame_changed(&mut self) -> bool {
            frame_has_changed(&self.bgra, &mut self.previous_bgra)
        }

        pub fn frame_dimensions(&self) -> (u32, u32) {
            (self.target_width as u32, self.target_height as u32)
        }

        pub fn nv12_frame(&mut self) -> &[u8] {
            convert_bgra_to_nv12(
                &self.bgra,
                self.target_width as usize,
                self.target_height as usize,
                &mut self.nv12,
            );
            &self.nv12
        }

        pub fn encode_jpeg(&mut self, quality: u8) -> Result<&[u8], String> {
            self.jpeg.clear();
            Encoder::new(&mut self.jpeg, quality.clamp(35, 85))
                .encode(
                    &self.bgra,
                    self.target_width as u16,
                    self.target_height as u16,
                    ColorType::Bgra,
                )
                .map_err(|error| format!("화면 JPEG 생성에 실패했습니다: {error}"))?;
            Ok(&self.jpeg)
        }

        pub fn capture_jpeg(&mut self, max_width: i32, quality: u8) -> Result<&[u8], String> {
            self.capture_frame(max_width)?;
            self.encode_jpeg(quality)
        }
    }

    impl Drop for ScreenCapture {
        fn drop(&mut self) {
            unsafe {
                if !self.memory_dc.is_null()
                    && !self.previous.is_null()
                    && self.previous as isize != -1
                {
                    SelectObject(self.memory_dc, self.previous);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                if !self.memory_dc.is_null() {
                    DeleteDC(self.memory_dc);
                }
                if !self.desktop_dc.is_null() {
                    ReleaseDC(std::ptr::null_mut(), self.desktop_dc);
                }
            }
        }
    }

    pub fn capture_jpeg(max_width: i32, quality: u8) -> Result<Vec<u8>, String> {
        let mut capture = ScreenCapture::new(max_width)?;
        Ok(capture.capture_jpeg(max_width, quality)?.to_vec())
    }

    fn scaled_dimensions(source: DesktopGeometry, max_width: i32) -> (i32, i32) {
        let target_width = source.width.min(max_width.max(320)) & !1;
        let target_height = (((source.height as i64 * target_width as i64) / source.width as i64)
            .max(2) as i32)
            & !1;
        (target_width, target_height)
    }

    fn convert_bgra_to_nv12(bgra: &[u8], width: usize, height: usize, nv12: &mut Vec<u8>) {
        debug_assert_eq!(bgra.len(), width * height * 4);
        debug_assert_eq!(width % 2, 0);
        debug_assert_eq!(height % 2, 0);
        let luma_length = width * height;
        nv12.resize(luma_length + luma_length / 2, 0);

        let (luma, chroma) = nv12.split_at_mut(luma_length);
        let source_stride = width * 4;
        for y in (0..height).step_by(2) {
            let source_pair = &bgra[y * source_stride..(y + 2) * source_stride];
            let (top_source, bottom_source) = source_pair.split_at(source_stride);
            let luma_pair = &mut luma[y * width..(y + 2) * width];
            let (top_luma, bottom_luma) = luma_pair.split_at_mut(width);
            let chroma_row = &mut chroma[(y / 2) * width..(y / 2 + 1) * width];

            for x in (0..width).step_by(2) {
                let pixel = x * 4;
                let (r00, g00, b00) = bgra_pixel(top_source, pixel);
                let (r01, g01, b01) = bgra_pixel(top_source, pixel + 4);
                let (r10, g10, b10) = bgra_pixel(bottom_source, pixel);
                let (r11, g11, b11) = bgra_pixel(bottom_source, pixel + 4);

                top_luma[x] = limited_luma(r00, g00, b00);
                top_luma[x + 1] = limited_luma(r01, g01, b01);
                bottom_luma[x] = limited_luma(r10, g10, b10);
                bottom_luma[x + 1] = limited_luma(r11, g11, b11);

                let average_red = (r00 + r01 + r10 + r11 + 2) / 4;
                let average_green = (g00 + g01 + g10 + g11 + 2) / 4;
                let average_blue = (b00 + b01 + b10 + b11 + 2) / 4;
                chroma_row[x] = limited_cb(average_red, average_green, average_blue);
                chroma_row[x + 1] = limited_cr(average_red, average_green, average_blue);
            }
        }
    }

    #[inline]
    fn bgra_pixel(row: &[u8], offset: usize) -> (i32, i32, i32) {
        (
            i32::from(row[offset + 2]),
            i32::from(row[offset + 1]),
            i32::from(row[offset]),
        )
    }

    fn frame_has_changed(current: &[u8], previous: &mut Vec<u8>) -> bool {
        if current == previous.as_slice() {
            return false;
        }
        previous.clear();
        previous.extend_from_slice(current);
        true
    }

    fn limited_luma(red: i32, green: i32, blue: i32) -> u8 {
        (((47 * red + 157 * green + 16 * blue + 128) >> 8) + 16).clamp(16, 235) as u8
    }

    fn limited_cb(red: i32, green: i32, blue: i32) -> u8 {
        (((-26 * red - 87 * green + 112 * blue + 128) >> 8) + 128).clamp(16, 240) as u8
    }

    fn limited_cr(red: i32, green: i32, blue: i32) -> u8 {
        (((112 * red - 102 * green - 10 * blue + 128) >> 8) + 128).clamp(16, 240) as u8
    }

    pub fn move_pointer(normalized_x: f64, normalized_y: f64) -> Result<(), String> {
        let screen = geometry()?;
        let x = screen.x + (normalized_x.clamp(0.0, 1.0) * (screen.width - 1) as f64) as i32;
        let y = screen.y + (normalized_y.clamp(0.0, 1.0) * (screen.height - 1) as f64) as i32;
        if unsafe { SetCursorPos(x, y) } == 0 {
            Err("마우스 위치를 옮기지 못했습니다.".to_string())
        } else {
            Ok(())
        }
    }

    pub fn mouse_button(button: &str, down: bool) -> Result<(), String> {
        let flag = match (button, down) {
            ("left", true) => MOUSEEVENTF_LEFTDOWN,
            ("left", false) => MOUSEEVENTF_LEFTUP,
            ("right", true) => MOUSEEVENTF_RIGHTDOWN,
            ("right", false) => MOUSEEVENTF_RIGHTUP,
            ("middle", true) => MOUSEEVENTF_MIDDLEDOWN,
            ("middle", false) => MOUSEEVENTF_MIDDLEUP,
            _ => return Err("지원하지 않는 마우스 버튼입니다.".to_string()),
        };
        unsafe { mouse_event(flag, 0, 0, 0, 0) };
        Ok(())
    }

    pub fn scroll(delta: i32) {
        unsafe { mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta.clamp(-1200, 1200), 0) };
    }

    pub fn scroll_horizontal(delta: i32) {
        unsafe { mouse_event(MOUSEEVENTF_HWHEEL, 0, 0, delta.clamp(-1200, 1200), 0) };
    }

    pub fn press_key(name: &str) -> Result<(), String> {
        let key = match name.to_ascii_lowercase().as_str() {
            "escape" | "esc" => VK_ESCAPE,
            "enter" | "return" => VK_RETURN,
            "space" => VK_SPACE,
            "tab" => VK_TAB,
            "backspace" => VK_BACK,
            "delete" => VK_DELETE,
            "left" => VK_LEFT,
            "right" => VK_RIGHT,
            "up" => VK_UP,
            "down" => VK_DOWN,
            "home" => VK_HOME,
            "end" => VK_END,
            "pageup" => VK_PRIOR,
            "pagedown" => VK_NEXT,
            "f1" => VK_F1,
            "f2" => VK_F2,
            "f3" => VK_F3,
            "f4" => VK_F4,
            "f5" => VK_F5,
            "f6" => VK_F6,
            "f7" => VK_F7,
            "f8" => VK_F8,
            "f9" => VK_F9,
            "f10" => VK_F10,
            "f11" => VK_F11,
            "f12" => VK_F12,
            _ => return Err("지원하지 않는 키입니다.".to_string()),
        } as u8;
        unsafe {
            keybd_event(key, 0, 0, 0);
            keybd_event(key, 0, KEYEVENTF_KEYUP, 0);
        }
        Ok(())
    }

    pub fn type_text(text: &str) -> Result<(), String> {
        let units: Vec<u16> = text.encode_utf16().take(2048).collect();
        let mut inputs = Vec::with_capacity(units.len() * 2);
        for unit in units {
            inputs.push(unicode_input(unit, 0));
            inputs.push(unicode_input(unit, KEYEVENTF_KEYUP));
        }
        if inputs.is_empty() {
            return Ok(());
        }
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<Input>() as i32,
            )
        };
        if sent == inputs.len() as u32 {
            Ok(())
        } else {
            Err("문자 입력 일부를 Windows에 전달하지 못했습니다.".to_string())
        }
    }

    pub fn show_heart_window() -> Result<(), String> {
        let title: Vec<u16> = "heart\0".encode_utf16().collect();
        let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if window.is_null() {
            return Err("HEART 창을 찾지 못했습니다.".to_string());
        }
        unsafe {
            ShowWindow(window, SW_RESTORE);
            SetForegroundWindow(window);
        }
        Ok(())
    }

    fn unicode_input(unit: u16, flags: u32) -> Input {
        Input {
            input_type: INPUT_KEYBOARD,
            data: InputData {
                keyboard: KeyboardInput {
                    virtual_key: 0,
                    scan: unit,
                    flags: KEYEVENTF_UNICODE | flags,
                    time: 0,
                    extra_info: 0,
                },
            },
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn capture_dimensions_preserve_aspect_ratio_and_minimum_width() {
            let source = DesktopGeometry {
                x: 0,
                y: 0,
                width: 1_920,
                height: 1_080,
            };

            assert_eq!(scaled_dimensions(source, 1_080), (1_080, 606));
            assert_eq!(scaled_dimensions(source, 100), (320, 180));
            assert_eq!(scaled_dimensions(source, 4_000), (1_920, 1_080));
        }

        #[test]
        fn bgra_to_nv12_has_expected_planes_and_limited_range() {
            let bgra = [0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 255, 0, 0, 255, 0, 0];
            let mut nv12 = Vec::new();
            convert_bgra_to_nv12(&bgra, 2, 2, &mut nv12);

            assert_eq!(nv12.len(), 6);
            assert_eq!(nv12[0], 16);
            assert_eq!(nv12[1], 235);
            assert!(nv12[4] >= 16 && nv12[4] <= 240);
            assert!(nv12[5] >= 16 && nv12[5] <= 240);
        }

        #[test]
        fn frame_change_tracker_is_exact_and_reuses_storage() {
            let mut previous = Vec::with_capacity(8);
            assert!(frame_has_changed(&[1, 2, 3, 4], &mut previous));
            let allocation = previous.as_ptr();
            assert!(!frame_has_changed(&[1, 2, 3, 4], &mut previous));
            assert!(frame_has_changed(&[1, 2, 3, 5], &mut previous));
            assert_eq!(allocation, previous.as_ptr());
        }

        #[test]
        fn benchmark_target_nv12_conversion_when_requested() {
            if std::env::var_os("HEART_TEST_NV12").is_none() {
                return;
            }
            let width = 1_728_usize;
            let height = 1_080_usize;
            let bgra = vec![127_u8; width * height * 4];
            let mut nv12 = Vec::new();
            let started = std::time::Instant::now();
            for _ in 0..120 {
                convert_bgra_to_nv12(&bgra, width, height, &mut nv12);
                std::hint::black_box(&nv12);
            }
            let elapsed = started.elapsed();
            eprintln!(
                "HEART BGRA→NV12 {}×{}: {:.1} fps ({:.2} ms/frame)",
                width,
                height,
                120.0 / elapsed.as_secs_f64(),
                elapsed.as_secs_f64() * 1000.0 / 120.0
            );
        }

        #[test]
        fn benchmark_exact_change_detection_when_requested() {
            if std::env::var_os("HEART_TEST_FRAME_COMPARE").is_none() {
                return;
            }
            let frame = vec![127_u8; 1_728 * 1_080 * 4];
            let mut previous = frame.clone();

            let comparisons = 240_u32;
            let started = std::time::Instant::now();
            for _ in 0..comparisons {
                std::hint::black_box(frame_has_changed(
                    std::hint::black_box(&frame),
                    &mut previous,
                ));
            }
            let elapsed = started.elapsed();
            eprintln!(
                "HEART exact frame comparison: {:.1} fps ({:.3} ms/frame)",
                f64::from(comparisons) / elapsed.as_secs_f64(),
                elapsed.as_secs_f64() * 1000.0 / f64::from(comparisons)
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_desktop::*;

#[cfg(not(target_os = "windows"))]
mod unsupported {
    #[derive(Debug, Clone, Copy)]
    pub struct DesktopGeometry {
        pub x: i32,
        pub y: i32,
        pub width: i32,
        pub height: i32,
    }

    fn unavailable<T>() -> Result<T, String> {
        Err("화면 원격 조작은 Windows에서만 지원합니다.".to_string())
    }

    pub fn geometry() -> Result<DesktopGeometry, String> {
        unavailable()
    }
    pub fn capture_jpeg(_: i32, _: u8) -> Result<Vec<u8>, String> {
        unavailable()
    }
    pub fn scroll_horizontal(_: i32) {}
    pub struct ScreenCapture;
    impl ScreenCapture {
        pub fn new(_: i32) -> Result<Self, String> {
            unavailable()
        }
        pub fn capture_jpeg(&mut self, _: i32, _: u8) -> Result<&[u8], String> {
            unavailable()
        }
        pub fn capture_frame(&mut self, _: i32) -> Result<(), String> {
            unavailable()
        }
        pub fn frame_changed(&mut self) -> bool {
            true
        }
        pub fn frame_dimensions(&self) -> (u32, u32) {
            (0, 0)
        }
        pub fn nv12_frame(&mut self) -> &[u8] {
            &[]
        }
        pub fn encode_jpeg(&mut self, _: u8) -> Result<&[u8], String> {
            unavailable()
        }
    }
    pub fn move_pointer(_: f64, _: f64) -> Result<(), String> {
        unavailable()
    }
    pub fn mouse_button(_: &str, _: bool) -> Result<(), String> {
        unavailable()
    }
    pub fn scroll(_: i32) {}
    pub fn press_key(_: &str) -> Result<(), String> {
        unavailable()
    }
    pub fn type_text(_: &str) -> Result<(), String> {
        unavailable()
    }
    pub fn show_heart_window() -> Result<(), String> {
        unavailable()
    }
}

#[cfg(not(target_os = "windows"))]
pub use unsupported::*;
