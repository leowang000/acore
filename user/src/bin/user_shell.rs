#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate user_lib;

use alloc::string::String;
use user_lib::console::getchar;
use user_lib::{exec, fork, waitpid};

const LF: u8 = 0x0au8; // '\n'
const CR: u8 = 0x0du8; // '\r'
const DL: u8 = 0x7fu8; // Delete
const BS: u8 = 0x08u8; // Backspace

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Rust user shell");
    print!(">> ");
    let mut line = String::new();
    loop {
        let c = getchar();
        match c {
            0x04 => { // Ctrl + D
                println!("\nShell exiting...");
                return 0;
            },
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    line.push('\0');
                    let pid = fork();
                    if pid == 0 {
                        // child process
                        if exec(line.as_str()) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!(">> ");
            }
            DL | BS => {
                print!("{0} {0}", BS as char);
                line.pop();
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
