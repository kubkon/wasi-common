use misc_tests::open_scratch_directory;
use misc_tests::utils::cleanup_file;
use misc_tests::wasi_wrappers::{wasi_path_open, wasi_path_symlink};
use std::{env, process};
use wasi::wasi_unstable;

fn test_symlink_loop(dir_fd: wasi_unstable::Fd) {
    // Create a self-referencing symlink.
    let mut status = wasi_path_symlink("symlink", dir_fd, "symlink");
    assert_eq!(status, wasi_unstable::ESUCCESS, "creating a symlink");

    // Try to open it.
    let mut file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(dir_fd, 0, "symlink", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::ELOOP,
        "opening a self-referencing symlink",
    );

    // Clean up.
    cleanup_file(dir_fd, "symlink");
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    test_symlink_loop(dir_fd)
}
