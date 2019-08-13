use libc;
use misc_tests::open_scratch_directory;
use misc_tests::utils::close_fd;
use misc_tests::wasi_wrappers::{wasi_fd_fdstat_get, wasi_path_open};
use std::{env, mem, process};
use wasi::wasi_unstable;

fn test_renumber(dir_fd: wasi_unstable::Fd) {
    let pre_fd: wasi_unstable::Fd = (libc::STDERR_FILENO + 1) as wasi_unstable::Fd;

    assert!(dir_fd > pre_fd, "dir_fd number");

    // Create a file in the scratch directory.
    let mut fd_from = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "file1",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut fd_from,
    );
    assert_eq!(status, wasi_unstable::ESUCCESS, "opening a file");
    assert!(
        fd_from > libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    // Get fd_from fdstat attributes
    let mut fdstat_from: wasi_unstable::FdStat = unsafe { mem::zeroed() };
    status = wasi_fd_fdstat_get(fd_from, &mut fdstat_from);
    assert_eq!(
        status,
        wasi_unstable::ESUCCESS,
        "calling fd_fdstat on the open file descriptor"
    );

    // Create another file in the scratch directory.
    let mut fd_to = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(
        dir_fd,
        0,
        "file2",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut fd_to,
    );
    assert_eq!(status, wasi_unstable::ESUCCESS, "opening a file");
    assert!(
        fd_to > libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    // Renumber fd of file1 into fd of file2
    status = wasi_unstable::fd_renumber(fd_from, fd_to);
    assert_eq!(
        status,
        wasi_unstable::ESUCCESS,
        "renumbering two descriptors",
    );

    // Ensure that fd_from is closed
    status = wasi_unstable::fd_close(fd_from);
    assert_eq!(
        status,
        wasi_unstable::EBADF,
        "closing already closed file descriptor"
    );

    // Ensure that fd_to is still open.
    let mut fdstat_to: wasi_unstable::FdStat = unsafe { mem::zeroed() };
    status = wasi_fd_fdstat_get(fd_to, &mut fdstat_to);
    assert_eq!(
        status,
        wasi_unstable::ESUCCESS,
        "calling fd_fdstat on the open file descriptor"
    );
    assert!(
        fdstat_from.fs_filetype == fdstat_to.fs_filetype
            && fdstat_from.fs_flags == fdstat_to.fs_flags
            && fdstat_from.fs_rights_base == fdstat_to.fs_rights_base
            && fdstat_from.fs_rights_inheriting == fdstat_to.fs_rights_inheriting,
        "expected fd_to have the same fdstat as fd_from",
    );

    close_fd(fd_to);
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
    test_renumber(dir_fd)
}
