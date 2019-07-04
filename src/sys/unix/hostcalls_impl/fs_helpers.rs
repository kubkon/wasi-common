#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use crate::ctx::WasiCtx;
use crate::fdentry::Descriptor;
use crate::host;
use crate::sys::host_impl::{self, RawString};

use nix::libc::{self, c_long};
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::prelude::{AsRawFd, FromRawFd};
use std::path::{Component, Path};

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub fn path_get(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &RawString,
    needed_base: host::__wasi_rights_t,
    needed_inheriting: host::__wasi_rights_t,
    needs_final_component: bool,
) -> Result<(File, RawString), host::__wasi_errno_t> {
    use nix::errno::Errno;
    use nix::fcntl::{openat, readlinkat, OFlag};
    use nix::sys::stat::Mode;

    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    if path.contains(&b'\0') {
        // if contains NUL, return EILSEQ
        return Err(host::__WASI_EILSEQ);
    }

    let dirfe = wasi_ctx.get_fd_entry(dirfd, needed_base, needed_inheriting)?;
    let dirfd = match &dirfe.fd_object.descriptor {
        Descriptor::File(f) => f.try_clone().expect("could clone dirfd"),
        _ => return Err(host::__WASI_EBADF),
    };

    // Stack of directory file descriptors. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a file descriptor to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfd];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![path.clone()];

    // Track the number of symlinks we've expanded, so we can return `ELOOP` after too many.
    let mut symlink_expansions = 0;

    // Buffer to read links into; defined outside of the loop so we don't reallocate it constantly.
    let mut readlink_buf = vec![0u8; libc::PATH_MAX as usize + 1];

    // TODO: rewrite this using a custom posix path type, with a component iterator that respects
    // trailing slashes. This version does way too much allocation, and is way too fiddly.
    loop {
        match path_stack.pop() {
            Some(cur_path) => {
                // eprintln!("cur_path = {:?}", cur_path);

                let ends_with_slash = cur_path.ends_with(b"/");
                let mut components = Path::new(&cur_path).components();
                let head = match components.next() {
                    None => return Err(host::__WASI_ENOENT),
                    Some(p) => p,
                };
                let tail = components.as_path();

                if tail.components().next().is_some() {
                    let mut tail = RawString::from(tail.as_os_str());
                    if ends_with_slash {
                        tail.push("/");
                    }
                    path_stack.push(tail);
                }

                match head {
                    Component::Prefix(_) | Component::RootDir => {
                        // path is absolute!
                        return Err(host::__WASI_ENOTCAPABLE);
                    }
                    Component::CurDir => {
                        // "." so skip
                        continue;
                    }
                    Component::ParentDir => {
                        // ".." so pop a dir
                        let _ = dir_stack.pop().expect("dir_stack is never empty");

                        // we're not allowed to pop past the original directory
                        if dir_stack.is_empty() {
                            return Err(host::__WASI_ENOTCAPABLE);
                        }
                    }
                    Component::Normal(head) => {
                        let mut head = RawString::from(head);
                        if ends_with_slash {
                            // preserve trailing slash
                            head.push("/");
                        }

                        if !path_stack.is_empty() || (ends_with_slash && !needs_final_component) {
                            match openat(
                                dir_stack.last().expect("dir_stack is never empty").as_raw_fd(),
                                head.as_ref(),
                                OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
                                Mode::empty(),
                            ) {
                                Ok(new_dir) => {
                                    dir_stack.push(unsafe { File::from_raw_fd(new_dir) });
                                    continue;
                                }
                                Err(e)
                                    // Check to see if it was a symlink. Linux indicates
                                    // this with ENOTDIR because of the O_DIRECTORY flag.
                                    if e.as_errno() == Some(Errno::ELOOP)
                                        || e.as_errno() == Some(Errno::EMLINK)
                                        || e.as_errno() == Some(Errno::ENOTDIR) =>
                                {
                                    // attempt symlink expansion
                                    match readlinkat(
                                        dir_stack.last().expect("dir_stack is never empty").as_raw_fd(),
                                        head.as_ref(),
                                        readlink_buf.as_mut_slice(),
                                    ) {
                                        Ok(link_path) => {
                                            symlink_expansions += 1;
                                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                                return Err(host::__WASI_ELOOP);
                                            }

                                            let mut link_path = RawString::from(link_path);
                                            if head.ends_with(b"/") {
                                                link_path.push("/");
                                            }

                                            path_stack.push(link_path);
                                            continue;
                                        }
                                        Err(e) => {
                                            return Err(
                                                host_impl::errno_from_nix(e.as_errno().unwrap()),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(
                                        host_impl::errno_from_nix(e.as_errno().unwrap()),
                                    );
                                }
                            }
                        } else if ends_with_slash
                            || (dirflags & host::__WASI_LOOKUP_SYMLINK_FOLLOW) != 0
                        {
                            // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                            // symlink expansion
                            match readlinkat(
                                dir_stack
                                    .last()
                                    .expect("dir_stack is never empty")
                                    .as_raw_fd(),
                                head.as_ref(),
                                readlink_buf.as_mut_slice(),
                            ) {
                                Ok(link_path) => {
                                    symlink_expansions += 1;
                                    if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                        return Err(host::__WASI_ELOOP);
                                    }
                                    let mut link_path = RawString::from(link_path);
                                    if head.ends_with(b"/") {
                                        link_path.push("/");
                                    }

                                    path_stack.push(link_path);
                                    continue;
                                }
                                Err(e) => {
                                    let errno = e.as_errno().unwrap();
                                    if errno != Errno::EINVAL && errno != Errno::ENOENT {
                                        // only return an error if this path is not actually a symlink
                                        return Err(host_impl::errno_from_nix(errno));
                                    }
                                }
                            }
                        }

                        // not a symlink, so we're done;
                        return Ok((
                            dir_stack.pop().expect("there is always a dirfd to return"),
                            head,
                        ));
                    }
                }
            }
            None => {
                // no further components to process. means we've hit a case like "." or "a/..", or if the
                // input path has trailing slashes and `needs_final_component` is not set
                return Ok((
                    dir_stack.pop().expect("there is always a dirfd to return"),
                    RawString::from(OsStr::new(".")),
                ));
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn utime_now() -> c_long {
    libc::UTIME_NOW
}

#[cfg(target_os = "macos")]
pub fn utime_now() -> c_long {
    -1
}

#[cfg(not(target_os = "macos"))]
pub fn utime_omit() -> c_long {
    libc::UTIME_OMIT
}

#[cfg(target_os = "macos")]
pub fn utime_omit() -> c_long {
    -2
}
