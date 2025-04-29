use std::ops::Range;

/// Valid only for 32-bit wasm!
#[derive(Debug, Clone, Copy)]
pub struct RawWasmVec {
    pub ptr: usize,
    pub len: usize,
}

impl From<u64> for RawWasmVec {
    fn from(value: u64) -> Self {
        let ptr = value as u32 as usize;
        let len = (value >> 32) as u32 as usize;
        Self { ptr, len }
    }
}

impl From<Vec<u8>> for RawWasmVec {
    fn from(value: Vec<u8>) -> Self {
        let ptr = value.as_ptr() as u32 as usize;
        let len = value.len() as u32 as usize;
        core::mem::forget(value);
        Self { ptr, len }
    }
}

impl Into<u64> for RawWasmVec {
    fn into(self) -> u64 {
        (self.len as u64) << 32 | self.ptr as u64
    }
}

impl RawWasmVec {
    pub fn into_range(&self) -> Range<usize> {
        self.ptr..self.ptr + self.len
    }
}
