#[cfg(target_os = "windows")]
pub fn desktop_available() -> bool {
    // Only inspect the input desktop; never switch or activate desktops.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenInputDesktop(flags: u32, inherit: i32, access: u32) -> isize;
        fn GetUserObjectInformationW(
            handle: isize,
            index: i32,
            info: *mut u16,
            length: u32,
            needed: *mut u32,
        ) -> i32;
        fn CloseDesktop(handle: isize) -> i32;
    }
    unsafe {
        let desktop = OpenInputDesktop(0, 0, 1);
        if desktop == 0 {
            return false;
        }
        let mut name = [0u16; 256];
        let mut needed = 0;
        let ok = GetUserObjectInformationW(desktop, 2, name.as_mut_ptr(), 512, &mut needed);
        CloseDesktop(desktop);
        ok != 0
            && String::from_utf16_lossy(
                &name[..name.iter().position(|c| *c == 0).unwrap_or(name.len())],
            )
            .eq_ignore_ascii_case("default")
    }
}
#[cfg(not(target_os = "windows"))]
pub fn desktop_available() -> bool {
    true
}

pub fn play_sound() {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "winmm")]
        unsafe extern "system" {
            fn PlaySoundW(sound: *const u16, module: isize, flags: u32) -> i32;
        }
        let alias: Vec<u16> = "SystemNotification".encode_utf16().chain(Some(0)).collect();
        unsafe {
            PlaySoundW(alias.as_ptr(), 0, 0x00010000 | 0x0001 | 0x0002);
        }
    }
}
