use std::env;
use std::process::Command;

fn assemble_file(input: &str, output: &str) {
    let status = Command::new("nasm")
        .args(&["-f", "elf64", input, "-o", output])
        .status()
        .expect(&format!("Failed to assemble {}", input));

    if !status.success() {
        panic!("Assembly of {} failed with status: {}", input, status);
    }
}


fn main() {
    let target_dir = env::var("OUT_DIR").unwrap();

    // Assemble ASM files
    let asm_files = ["boot.asm", "boot-64.asm", "header.asm"];
    for file in &asm_files {
        let input_path = format!("bootloader/{}", file);
        let output_path = format!("{}/{}.o", target_dir, file);

        assemble_file(&input_path, &output_path);
        println!("cargo:rustc-link-arg={}", output_path);
    }

    // Custom linker arguments
    println!("cargo:rustc-link-arg=-Tlinker.ld");
}
