#![no_std]
#![no_main]

extern crate alloc;
use core::panic::PanicInfo;

use anasos_kernel::{
    allocator, framebuffer, framebuffer_off,
    // framebuffer_theseus::{
    //     self, color,
    //     pixel::{AlphaPixel, Pixel, RGBPixel},
    //     Framebuffer,
    // },
    hlt, init,
    memory::{
        self, create_example_mapping, is_identity_mapped,
        memory_map::{FrameRange, FromMemoryMapTag, MemoryMap, MemoryRegion, MemoryRegionType},
        BootInfoFrameAllocator,
    },
    // pci_controller::{self, enumerate_pci_devices, print_pci_devices},
    println, serial_println,
    task::{executor::Executor, keyboard, Task},
};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder},
    text::Text,
};
use x86_64::{
    // structures::paging::{frame, Translate},
    structures::paging::Translate, PhysAddr, VirtAddr
};

extern crate multiboot2;
use multiboot2::{BootInformation, BootInformationHeader};

#[no_mangle]
pub extern "C" fn _start(mb_magic: u32, mbi_ptr: u32) -> ! {
    if mb_magic != multiboot2::MAGIC {
        panic!("Invalid Multiboot2 magic number");
    }

    let boot_info =
        unsafe { BootInformation::load(mbi_ptr as *const BootInformationHeader).unwrap() };
    let _cmd = boot_info.command_line_tag();

    if let Some(bootloader_name) = boot_info.boot_loader_name_tag() {
        println!("Bootloader: {:?}", bootloader_name.name().ok());
    }

    // Access the memory map
    if let Some(memory_map_tag) = boot_info.memory_map_tag() {
        for area in memory_map_tag.memory_areas() {
            println!(
                "Memory area: start = {:#x}, length = {:#x}, type = {:?}",
                area.start_address(),
                area.size(),
                area.typ()
            );
        }
    }

    println!("");

    // #[cfg(notest)]
    kernel_main(&boot_info);

    // #[cfg(test)]
    // test_kernel_main(&*BOOT_INFO);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panicked: \n{}", info);
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);

    hlt();
}

fn kernel_main(boot_info: &BootInformation) -> ! {
    println!("Kernel Start:");
    //  // Check MMIO status for the framebuffer
    //  let pci_bus = 0; // Replace with your framebuffer's bus number
    //  let pci_slot = 2; // Replace with your framebuffer's slot number
    //  let pci_function = 0; // Replace with your framebuffer's function number

    //  let command_register = read_pci_register(pci_bus, pci_slot, pci_function, 0x04);
    //  println!("PCI command register: {:#b}", command_register);
    //  if (command_register & (1 << 1)) != 0 {
    //      println!("MMIO is enabled for framebuffer.");
    //  } else {
    //      println!("MMIO is NOT enabled for framebuffer. Enabling it...");

    //     //  let mut new_command = command_register | (1 << 1); // Enable MMIO
    //     //  write_pci_register(pci_bus, pci_slot, pci_function, 0x04, new_command);
    //     //  println!("MMIO enabled.");
    //  }

    init();

    // println!("boot_info start");
    // println!("{:#?}", boot_info);
    // println!("boot_info end");

    
    let phys_mem_offset = VirtAddr::new(boot_info.end_address() as u64);
    // let phys_mem_offset = VirtAddr::new(kernel_start as u64);
    println!("Physical memory offset: {:?}", phys_mem_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // println!("Mapper : {:#?}", mapper);
    let mut memory_map: MemoryMap =
        MemoryMap::from_memory_map_tag(boot_info.memory_map_tag().unwrap());

    let framebuffer_tag = boot_info
        .framebuffer_tag()
        .unwrap()
        .ok()
        .ok_or("No framebuffer tag found")
        .unwrap();
    
    let framebuffer_phys_addr = PhysAddr::new(framebuffer_tag.address());
    println!("Framebuffer physical address: {:?}", framebuffer_phys_addr);
    let framebuffer_virt_addr = VirtAddr::new(framebuffer_phys_addr.as_u64()); // Example virtual address
    
    let framebuffer_start = framebuffer_tag.address() as u64;
    let framebuffer_size = framebuffer_tag.pitch() as u64 * framebuffer_tag.height() as u64;
    let framebuffer_width = framebuffer_tag.width() as u64;
    let framebuffer_height = framebuffer_tag.height() as u64;
    
    println!("Framebuffer start: {:#x}", framebuffer_start);
    println!("Framebuffer size: {}", framebuffer_size);
    println!("Framebuffer width: {}", framebuffer_width);
    println!("Framebuffer end: {:#x}", framebuffer_start + framebuffer_size);
    println!("Framebuffer height: {}", framebuffer_height);

    // reserve framebuffer memory
    memory_map.add_region(MemoryRegion {
        range: FrameRange::new(framebuffer_start, framebuffer_start + framebuffer_size),
        region_type: MemoryRegionType::Reserved,
    });


    // Calculate total pages usable
    let total_pages: u64 = memory_map.iter()
    .filter(|region| region.region_type == MemoryRegionType::Usable)
    .map(|region| {
        let start = region.range.start_addr();
        let end = region.range.end_addr();
        (end - start) / 4096 // 4 KiB page size
    })
    .sum();

    println!("Total pages required: {}", total_pages);


    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&mut memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");
    println!("Heap initialized");
    // VALID HEAP ALLOCATION STARTS HERE



    // {
    //     // scope is needed to drop the mutable reference to the heap allocator
    //     let devices = enumerate_pci_devices();
    //     print_pci_devices(&devices);
    // }
    // println!("PCI devices enumerated {:#?}", framebuffer_tag);


    println!("");
    println!(
        "Framebuffer width: {}, height: {}",
        framebuffer_width, framebuffer_height
    );

    println!("");

    framebuffer::map_framebuffer(
        framebuffer_phys_addr,
        framebuffer_size,
        VirtAddr::new(framebuffer_phys_addr.as_u64()),
        &mut mapper,
        &mut frame_allocator,
    )
    .expect("Framebuffer mapping failed");

    println!("Framebuffer mapped");

    framebuffer::check_framebuffer_mapping(&mut mapper, framebuffer_tag);
    if is_identity_mapped(VirtAddr::new(framebuffer_phys_addr.as_u64()), &mapper) {
        println!(
            "Framebuffer identity mapped to address: {:x}",
            framebuffer_phys_addr.as_u64()
        );
    } else {
        println!("Framebuffer not identity mapped");
    }
    println!("");

    let virt_addr = VirtAddr::new(0xfd000000);
    if let Some(phys_addr) = mapper.translate_addr(virt_addr) {
        println!("Address {:#x} maps to physical address {:#x}", virt_addr.as_u64(), phys_addr.as_u64());
    } else {
        println!("Address {:#x} is not mapped", virt_addr.as_u64());
    }

    unsafe {
        println!("Pixel value before: {:#x}", read_from_mmio(framebuffer_virt_addr.as_u64()));
        // write_to_mmio(mmio_address, 0x12345678);
        *(framebuffer_virt_addr.as_mut_ptr::<u32>()) = 0x00FF00; // Set to green
        println!("Pixel value after: {:#x}", *(framebuffer_virt_addr.as_mut_ptr::<u32>()));
    }

    let mut framebuffer = framebuffer_off::Framebuffer::new(
        framebuffer_height as usize,
        framebuffer_width as usize,
        unsafe {
            core::slice::from_raw_parts_mut(
                0xfd000000 as *mut u8,
                framebuffer_height as usize
                    * framebuffer_width as usize
                    * (framebuffer_tag.bpp() / 8) as usize,
            )
        },
    );

    // Draw a circle
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb888::RED)
        .stroke_width(1)
        .fill_color(Rgb888::GREEN)
        .build();

    Circle::new(Point::new(100, 100), 50)
        .into_styled(style)
        .draw(&mut framebuffer)
        .unwrap();

    // Draw text
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X9)
        .text_color(Rgb888::WHITE)
        .build();

    Text::new("Hello, OS!", Point::new(10, 10), text_style)
        .draw(&mut framebuffer)
        .unwrap();

    // println!("Framebuffer identity mapped to address: {:?}", framebuffer.buffer_mut().as_ptr());

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run(); // This function will never return
}

fn test_kernel_main(_boot_info: &BootInformation) -> ! {
    println!("Running tests");
    // test code here
    println!("Tests passed");

    hlt();
}

// Test async functions
async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

// fn enumerate_pci_devices() {
//     for bus in 0..=255 {
//         for device in 0..=31 {
//             for function in 0..=7 {
//                 let vendor_id = read_pci_register(bus, device, function, 0x00) & 0xFFFF;
//                 if vendor_id != 0xFFFF {
//                     let device_id = (read_pci_register(bus, device, function, 0x00) >> 16) & 0xFFFF;
//                     println!(
//                         "Found device: bus = {}, device = {}, function = {}, vendor = {:#X}, device = {:#X}",
//                         bus, device, function, vendor_id, device_id
//                     );
//                 }
//             }
//         }
//     }
// }

// fn make_screen_green(framebuffer: *mut u32, width: u32, height: u32, pitch: u32, bpp: u8) {
//     let green_color: u32 = 0x00FF0000; // Green in 32-bit ARGB

//     unsafe {
//         for y in 0..height {
//             for x in 0..width {
//                 println!("x: {}, y: {}", x, y);
//                 let pixel_offset: u32 = (y * pitch + x * ((bpp as u32) / 8)).into();
//                 println!("pixel_offset: {}", pixel_offset);
//                 let pixel_ptr = framebuffer.add(pixel_offset as usize);
//                 println!("pixel_ptr: {:?}", pixel_ptr);
//                 println!("pixel_ptr value: {:?}", *pixel_ptr);

//                 *pixel_ptr = green_color;
//                 println!("After pixel_ptr value: {:?}", *pixel_ptr);
//             }
//         }
//     }
// }

// framebuffer::map_framebuffer_page_table(&mut mapper, &mut frame_allocator, framebuffer_tag);
// framebuffer::check_framebuffer_mapping(&mapper, framebuffer_tag);

// unsafe {
//     println!("Pixel value before: {:#x}", *(framebuffer_virt_addr.as_mut_ptr::<u32>()));
//     *(framebuffer_virt_addr.as_mut_ptr::<u32>()) = 0x00FF00; // Set to green
//     println!("Pixel value after: {:#x}", *(framebuffer_virt_addr.as_mut_ptr::<u32>()));
// }

//framebuffer::init(&framebuffer_tag, &mut mapper, &mut frame_allocator);

// Initialize the framebuffer
// let mut framebuffer: Framebuffer<AlphaPixel> = Framebuffer::new(
//     framebuffer_tag.width() as usize,
//     framebuffer_tag.height() as usize,
//     Some(framebuffer_phys_addr),
//     &mut mapper,
//     &mut frame_allocator,
// )
// .expect("Failed to initialize framebuffer");

// Fill the screen with green
// let green_pixel: AlphaPixel = color::GREEN.into();
// framebuffer.overwrite_pixel(3, 3, green_pixel);
// framebuffer.fill(green_pixel);

// println!("Screen filled with green color.");

// unsafe {
//     core::arch::asm!(
//         "mov eax, 0x00FF00",     // Green color
//         "mov [0xFD000000], eax", // Write to framebuffer address
//     );
// }


use core::ptr;

pub fn write_to_mmio(addr: u64, value: u32) {
    unsafe {
        ptr::write_volatile(addr as *mut u32, value);
    }
}

pub fn read_from_mmio(addr: u64) -> u32 {
    unsafe { ptr::read_volatile(addr as *const u32) }
}
