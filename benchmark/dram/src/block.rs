use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicUsize},
        LazyLock, Mutex,
    },
};

use errore::kind::{NOT_ALLOWED, NO_MEMORY};

use crate::{throw_rerr, Result};

pub struct GlobalConf {
    pub page_size: usize,
    pub huge_enabled: bool,
}

static MEM_ARGS: once_cell::sync::OnceCell<GlobalConf> = once_cell::sync::OnceCell::new();

pub static BYTES_MOVEMENT: AtomicU64 = AtomicU64::new(0);

pub fn init(f: impl FnOnce() -> GlobalConf) {
    MEM_ARGS.get_or_init(f);
}

pub fn conf() -> &'static GlobalConf {
    MEM_ARGS
        .get()
        .expect("Memory configuration not initialized")
}

pub struct MemSegment {
    node_id: u16,
    segment_id: u16,
    next_block_id: AtomicU32,
    vaddr: NonNull<u8>,
    elem_size: usize,
    num_elems: usize,
    size: usize,
}

impl MemSegment {
    pub fn new(node_id: u16, num_elems: usize, elem_size: usize) -> Result<Self> {
        const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;
        const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;
        assert_ne!(num_elems * elem_size, 0);
        let aligned_memory_size = align_with_page_size(num_elems * elem_size);
        let mut vaddr = next_base_vaddr(aligned_memory_size);
        let mut ctx = NUMA_CONTEXT.lock().unwrap();
        unsafe {
            ctx.set_policy(node_id)?;
            loop {
                let mut flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED;
                if conf().huge_enabled {
                    flags |= libc::MAP_HUGETLB;
                    if conf().page_size == PAGE_SIZE_1G {
                        flags |= libc::MAP_HUGE_1GB;
                    } else if conf().page_size == PAGE_SIZE_2M {
                        flags |= libc::MAP_HUGE_2MB;
                    } else {
                        throw_rerr!(
                            NOT_ALLOWED,
                            "Unsupported huge page size {}",
                            conf().page_size
                        );
                    }
                }

                let ret = libc::mmap(
                    vaddr.as_ptr() as *mut libc::c_void,
                    aligned_memory_size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    flags,
                    -1,
                    0,
                );

                if !ret.is_null() && ret != vaddr.as_ptr() as *mut libc::c_void {
                    libc::munmap(ret, aligned_memory_size);
                }

                if ret == vaddr.as_ptr() as *mut libc::c_void {
                    break;
                }

                vaddr = next_base_vaddr(aligned_memory_size);
            }
            ctx.release()?;

            Ok(Self {
                node_id,
                segment_id: Self::next_segment_id(),
                next_block_id: AtomicU32::new(0),
                vaddr,
                elem_size: elem_size,
                num_elems: num_elems,
                size: aligned_memory_size,
            })
        }
    }

    pub fn random_alloc(&self, size: usize) -> MemBlock {
        assert!(size <= self.elem_size);
        let offset = (rand::random::<usize>() % self.num_elems) * self.elem_size;
        let next_block_id = self
            .next_block_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        MemBlock {
            node_id: self.node_id,
            len: size as u16,
            segment_id: self.segment_id as u32,
            block_id: (self.node_id as u64) << 48
                | (self.segment_id as u64) << 32
                | next_block_id as u64,
            start: unsafe { NonNull::new_unchecked(self.vaddr.as_ptr().add(offset)) },
        }
    }

    pub fn sequence_alloc(&self, order: usize, size: usize) -> MemBlock {
        let order = order % self.num_elems;
        assert!(size <= self.elem_size);
        let offset = order * self.elem_size;
        let next_block_id = self
            .next_block_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        MemBlock {
            node_id: self.node_id,
            len: size as u16,
            segment_id: self.segment_id as u32,
            block_id: (self.node_id as u64) << 48
                | (self.segment_id as u64) << 32
                | next_block_id as u64,
            start: unsafe { NonNull::new_unchecked(self.vaddr.as_ptr().add(offset)) },
        }
    }

    fn next_segment_id() -> u16 {
        static SEGMENT_ID: AtomicU16 = AtomicU16::new(0);
        SEGMENT_ID.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }
}

impl Drop for MemSegment {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.vaddr.as_ptr() as *mut libc::c_void, self.size);
        }
    }
}

#[repr(align(32))]
pub struct MemBlock {
    node_id: u16,
    len: u16,
    segment_id: u32,
    block_id: u64,
    start: NonNull<u8>,
}

impl MemBlock {
    /// move data from this block to another block, return the number of bytes moved
    pub fn move_to(&self, block: &MemBlock) -> usize {
        log::trace!("Moving block {} (node_id={},segment_id={},len={}) to block {} (node_id={},segment_id={},len={})", 
        self.block_id,self.node_id,self.segment_id,self.len, block.block_id,block.node_id,block.segment_id,block.len);
        assert_ne!(self.segment_id, block.segment_id);
        unsafe {
            let len = usize::min(self.len as usize, block.len as usize);
            let mut remaining = len;
            let mut src = self.start.as_ptr();
            let mut dst = block.start.as_ptr();
            loop {
                if remaining == 0 {
                    break;
                }

                if remaining >= 64 {
                    std::ptr::copy(src, dst, 64);
                    remaining -= 64;
                    src = src.add(64);
                    dst = dst.add(64);
                } else {
                    std::ptr::copy(src, dst, remaining);
                    break;
                }
            }
            BYTES_MOVEMENT.fetch_add(len as u64, std::sync::atomic::Ordering::Relaxed);
            len
        }
    }
}

fn align_with_page_size(value: usize) -> usize {
    let page_size = conf().page_size;
    let ret = (value + page_size - 1) & !(page_size - 1);
    assert_eq!(ret % page_size, 0);
    ret
}

fn next_base_vaddr(size: usize) -> NonNull<u8> {
    static NEXT_BASE_VADDR: AtomicUsize = AtomicUsize::new(1 << 32);
    let vaddr = NEXT_BASE_VADDR.fetch_add(size, std::sync::atomic::Ordering::AcqRel);
    unsafe { NonNull::new_unchecked(vaddr as *mut u8) }
}

struct MemAllocatePolicy {
    mask: Option<NonNull<numa_sys::bitmask>>,
    owner: Option<libc::pthread_t>,
}

unsafe impl Send for MemAllocatePolicy {}

impl MemAllocatePolicy {
    fn set_policy(&mut self, node_id: u16) -> Result<()> {
        if !numa_available() {
            return Ok(());
        }

        assert_eq!(self.owner, None);

        unsafe {
            numa_sys::numa_set_strict(1);
            let mut new_bitmask = numa_sys::numa_allocate_nodemask();
            if new_bitmask.is_null() {
                throw_rerr!(
                    NO_MEMORY,
                    "Failed to allocate nodemask (error:{})",
                    std::ffi::CStr::from_ptr(libc::strerror(*libc::__errno_location()))
                        .to_str()
                        .unwrap()
                );
            }
            new_bitmask = numa_sys::numa_bitmask_clearall(new_bitmask);
            new_bitmask = numa_sys::numa_bitmask_setbit(new_bitmask, node_id as u32);
            let old_bitmask = numa_sys::numa_get_membind();
            numa_sys::numa_set_membind(new_bitmask);
            log::info!("Set NUMA policy: node_id = {}", node_id);
            // numa_sys::numa_bitmask_free(new_bitmask);

            self.mask = Some(NonNull::new_unchecked(old_bitmask));
            self.owner = Some(libc::pthread_self());
            Ok(())
        }
    }

    fn release(&mut self) -> Result<()> {
        if !numa_available() {
            return Ok(());
        }

        unsafe {
            assert_eq!(self.owner, Some(libc::pthread_self()));
            let new_bitmask = numa_sys::numa_get_membind();
            numa_sys::numa_set_membind(self.mask.take().unwrap().as_ptr());
            numa_sys::numa_bitmask_free(new_bitmask);
            self.owner.take();
        }

        Ok(())
    }
}

static NUMA_CONTEXT: LazyLock<Mutex<MemAllocatePolicy>> = LazyLock::new(|| {
    Mutex::new(MemAllocatePolicy {
        mask: None,
        owner: None,
    })
});

fn numa_available() -> bool {
    unsafe { numa_sys::numa_available() != 0 }
}
