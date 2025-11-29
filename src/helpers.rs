use std::mem::MaybeUninit;

pub fn vec_to_uninit_mut(vec: &mut Vec<u8>) -> &mut [MaybeUninit<u8>] {
    unsafe {
        std::slice::from_raw_parts_mut(vec.as_mut_ptr().cast::<MaybeUninit<u8>>(), vec.capacity())
    }
}

pub fn slice_to_uninit_mut(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    unsafe { &mut *(std::ptr::from_mut::<[u8]>(slice) as *mut [MaybeUninit<u8>]) }
}
