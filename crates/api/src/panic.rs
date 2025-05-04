#[cfg(not(test))]
mod panic_handler {
    use core::panic::PanicInfo;

    extern crate alloc;
    use alloc::string::ToString;

    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        let message = info.to_string();
        let message = message.as_bytes();
        unsafe { crate::external::panic(message.as_ptr() as u32, message.len() as u32) }
    }
}
