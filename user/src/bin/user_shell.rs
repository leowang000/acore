#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate user_lib;

use alloc::string::String;
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{close, dup, exec, fork, open, pipe, waitpid, OpenFlags};

#[derive(Debug)]
struct ProcessArguments {
    input: String,
    output: String,
    args_copy: Vec<String>,
    args_addr: Vec<*const u8>,
}

impl ProcessArguments {
    pub fn new(command: &str) -> Self {
        let args: Vec<_> = command.split(' ').collect();
        let mut args_copy: Vec<_> = args
            .iter()
            .filter(|arg| !arg.is_empty())
            .map(|arg| {
                let mut string = String::new();
                string.push_str(arg);
                string.push('\0');
                string
            })
            .collect();
        let mut input = String::new();
        if let Some((id, _)) = args_copy
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == "<\0")
        {
            input = args_copy[id + 1].clone();
            args_copy.drain(id..=id + 1);
        }
        let mut output = String::new();
        if let Some((id, _)) = args_copy
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == ">\0")
        {
            output = args_copy[id + 1].clone();
            args_copy.drain(id..=id + 1);
        }
        let mut args_addr: Vec<*const u8> =
            args_copy.iter().map(|arg: &String| arg.as_ptr()).collect();
        args_addr.push(core::ptr::null());
        Self {
            input: input,
            output: output,
            args_copy: args_copy,
            args_addr: args_addr,
        }
    }
}

/// '\n'
const LF: u8 = 0x0au8;
/// '\r'
const CR: u8 = 0x0du8;
/// Delete
const DL: u8 = 0x7fu8;
/// Backspace
const BS: u8 = 0x08u8;
const LINE_START: &str = ">> ";

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Rust user shell");
    print!(">> ");
    let mut line = String::new();
    loop {
        let c = getchar();
        match c {
            0x04 => {
                // Ctrl + D
                println!("\nShell exiting...");
                return 0;
            }
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    // Parse the arguments of each command.
                    let splited: Vec<&str> = line.as_str().split('|').collect();
                    let process_arguments_list: Vec<ProcessArguments> = splited
                        .iter()
                        .map(|cmd| ProcessArguments::new(cmd))
                        .collect();
                    let process_arguments_list_len = process_arguments_list.len();
                    // Validate the commands.
                    let mut valid = true;
                    if process_arguments_list_len == 1 {
                        valid = true;
                    } else {
                        for (i, process_arguments) in process_arguments_list.iter().enumerate() {
                            if i == 0 {
                                if !process_arguments.output.is_empty() {
                                    valid = false;
                                }
                            } else if i == process_arguments_list_len - 1 {
                                if !process_arguments.input.is_empty() {
                                    valid = false;
                                }
                            } else {
                                if !process_arguments.input.is_empty()
                                    || !process_arguments.output.is_empty()
                                {
                                    valid = false;
                                }
                            }
                        }
                    }
                    if !valid {
                        println!("Invalid command: Inputs/Outputs cannot be correctly binded!");
                    } else {
                        let mut pipes: Vec<[usize; 2]> = Vec::new();
                        if !process_arguments_list.is_empty() {
                            for _ in 0..process_arguments_list_len - 1 {
                                let mut pipe_fd = [0usize; 2];
                                pipe(&mut pipe_fd);
                                pipes.push(pipe_fd);
                            }
                        }
                        let mut children: Vec<usize> = Vec::new();
                        for (i, process_argument) in process_arguments_list.into_iter().enumerate()
                        {
                            let pid = fork();
                            if pid == 0 {
                                let input = process_argument.input;
                                let output = process_argument.output;
                                let args_copy = process_argument.args_copy;
                                let args_addr = process_argument.args_addr;
                                if !input.is_empty() {
                                    let input_fd = open(input.as_str(), OpenFlags::RDONLY);
                                    if input_fd == -1 {
                                        println!("Error when opening file{}", input);
                                        return -4;
                                    }
                                    let input_fd = input_fd as usize;
                                    close(0);
                                    assert_eq!(dup(input_fd), 0);
                                    close(input_fd);
                                }
                                if !output.is_empty() {
                                    let output_fd = open(
                                        output.as_str(),
                                        OpenFlags::CREATE | OpenFlags::WRONLY,
                                    );
                                    if output_fd == -1 {
                                        println!("Error when opening file{}", output);
                                        return -4;
                                    }
                                    let output_fd = output_fd as usize;
                                    close(1);
                                    assert_eq!(dup(output_fd), 1);
                                    close(output_fd);
                                }
                                if i > 0 {
                                    close(0);
                                    let read_end = pipes[i - 1][0];
                                    assert_eq!(dup(read_end), 0);
                                }
                                if i < process_arguments_list_len - 1 {
                                    close(1);
                                    let read_end = pipes[i][1];
                                    assert_eq!(dup(read_end), 1);
                                }
                                for pipe_fd in pipes.iter() {
                                    close(pipe_fd[0]);
                                    close(pipe_fd[1]);
                                }
                                if exec(args_copy[0].as_str(), args_addr.as_slice()) == -1 {
                                    println!("Error when executing!");
                                    return -4;
                                }
                                unreachable!();
                            } else {
                                children.push(pid as usize);
                            }
                        }
                        for pipe_fd in pipes.iter() {
                            close(pipe_fd[0]);
                            close(pipe_fd[1]);
                        }
                        let mut exit_code: i32 = 0;
                        for pid in children.into_iter() {
                            let exit_pid = waitpid(pid, &mut exit_code);
                            assert_eq!(pid, exit_pid as usize);
                            println!("Shell: Process {} exited with code {}", pid, exit_code);
                        }
                    }
                    line.clear();
                }
                print!("{}", LINE_START);
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
