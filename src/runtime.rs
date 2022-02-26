use core::alloc::Layout;
use core::arch::{asm, global_asm};
use core::hint::spin_loop;
use core::panic::PanicInfo;
use core::sync::atomic::*;
use linked_list_allocator::LockedHeap;

#[no_mangle]
pub extern "C" fn init(hartid: usize, dtb: usize) {
    static READY: AtomicBool = AtomicBool::new(false);
    if hartid == 0 {
        crate::log::init();
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(HEAP.as_ptr() as usize, HEAP_SIZE);
        }
        READY.store(true, Ordering::Release);
    } else {
        while !READY.load(Ordering::Acquire) {
            spin_loop();
        }
    }
    unsafe {
        main(hartid, dtb);
    }
    crate::sbi::legacy::shutdown();
}

extern "C" {
    fn main(hartid: usize, dtb: usize);
}

global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    la sp, bootstacktop

    call init

    .section .bss.stack
    .align 12
    .global bootstack
bootstack:
    .space 4096 * 16
    .global bootstacktop
bootstacktop:
"#
);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!("abort!");
}

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 0x1_00000;

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    panic!("out of memory: {:#x?}", layout);
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
