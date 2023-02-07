use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

// Frame allocator created from the memory map provided by the BootInfo struct from the
// bootloader.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    // Creates a frame allocator from the given memory map
    //
    // This function is unsafe because the caller has to guarantee that the `USABLE` memory regions
    // given by the memory map are in fact usable.
    pub unsafe fn init(mmap: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map: mmap,
            next: 0,
        }
    }
    // Returns an iterator over the usable frames specified in the memory map.
    pub fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let memory_regions = self.memory_map.iter();
        // filter only the regions marked `USABLE`
        let usable_regions = memory_regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_range = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let phy_frame_addresses = addr_range.flat_map(|a| a.step_by(4096));
        phy_frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    // This functoin just returns a usable frame
    //
    // In the context of mapping a Virtual Page to a Physical Frame, this function is used when
    // there is a need to create a new PageTable (because the pagetable does not exist). This
    // function provides a usable frame that can be used for the pagetable to be created.
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

// Initialize a new OffsetPageTable
//
// This function is unsafe because the caller must guarantee that the
// complete physical memory is mapped to virtual memory at the passed
// `physical_memory_offset`. Also, this function must be only called once
// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(phy_mem_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4_pt = active_level4_page_table(phy_mem_offset);
    OffsetPageTable::new(l4_pt, phy_mem_offset)
}

//. Returns a mutable reference to the active level 4 page table
//
// This function is unsafe because the caller must guarantee that the physical memory is mapped to
// the virtual memory and the offset provided points to the physical memory. Also, this function must be only called once
// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level4_page_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    // get the level 4 page table physical frame address from the CR3 register
    let (level4_pagetable_frame, _) = Cr3::read();
    // calculate the virtual address that points to the address of the l4 page table
    let p4_phy_addr = physical_memory_offset + level4_pagetable_frame.start_address().as_u64();
    // this gives a mutable raw pointer
    let p4_pointer: *mut PageTable = p4_phy_addr.as_mut_ptr();
    &mut *p4_pointer
}

// Translates the given virtual address to the physical address
//
// This function is unsafe because the caller must guarantee that the virtual memory region is
// mapped with the physical memory at a the given offset (phy_mem_offset)
pub unsafe fn translate_addr(addr: VirtAddr, phy_mem_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, phy_mem_offset)
}

// This is called by the translate_addr fn. This is done to limit the scope of unsafe in the
// translate_addr fn.
fn translate_addr_inner(addr: VirtAddr, phy_mem_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::page_table::FrameError;

    // read the active level 4 frame from the CR3 register
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    // traverse the multi-level page table
    for &index in &table_indexes {
        // convert the frame into a page table reference
        let virt = phy_mem_offset + frame.start_address().as_u64();
        // this gives us an immutable raw pointer (note that *const is just the syntax for immutabe
        // raw pointer type and not a dereference operator in this case)
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        // read the page table entry and update `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // calculate the physical address by adding the page offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
