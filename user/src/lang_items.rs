use crate::{getpid, kill, println, SIGABRT};

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message());
    }
    kill(getpid() as usize, SIGABRT);
    unreachable!();
}
