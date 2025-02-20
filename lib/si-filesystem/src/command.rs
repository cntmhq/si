use std::{ffi::OsString, path::PathBuf};

use fuser::{
    ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyLock, ReplyOpen, ReplyWrite, ReplyXattr,
};
use nix::libc::c_int;

use crate::{FileHandle, Inode};

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum FilesystemCommand {
    HydrateChangeSets,
    GetAttr {
        ino: Inode,
        fh: Option<FileHandle>,
        reply: ReplyAttr,
    },
    ReadDir {
        ino: Inode,
        fh: FileHandle,
        offset: i64,
        reply: ReplyDirectory,
    },
    Lookup {
        parent: Inode,
        name: OsString,
        reply: ReplyEntry,
    },
    Forget {
        ino: Inode,
        nlookup: u64,
    },
    SetAttr {
        ino: Inode,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        fh: Option<FileHandle>,
        flags: Option<u32>,
        reply: ReplyAttr,
    },
    ReadLink {
        ino: Inode,
        reply: ReplyData,
    },
    MkNod {
        parent: Inode,
        name: OsString,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    },
    MkDir {
        parent: Inode,
        name: OsString,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    },
    Unlink {
        parent: Inode,
        name: OsString,
        reply: ReplyEmpty,
    },
    RmDir {
        parent: Inode,
        name: OsString,
        reply: ReplyEmpty,
    },
    SymLink {
        parent: Inode,
        link_name: OsString,
        target: PathBuf,
        reply: ReplyEntry,
    },
    Rename {
        parent: Inode,
        name: OsString,
        newparent: Inode,
        newname: OsString,
        flags: u32,
        reply: ReplyEmpty,
    },
    Link {
        ino: Inode,
        newparent: Inode,
        newname: OsString,
        reply: ReplyEntry,
    },
    Open {
        ino: Inode,
        flags: i32,
        reply: ReplyOpen,
    },
    Read {
        ino: Inode,
        fh: FileHandle,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    },
    Write {
        ino: Inode,
        fh: FileHandle,
        offset: i64,
        data: Vec<u8>,
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    },
    Flush {
        ino: Inode,
        fh: FileHandle,
        lock_owner: u64,
        reply: ReplyEmpty,
    },
    Release {
        ino: Inode,
        fh: FileHandle,
        flags: i32,
        lock_owner: Option<u64>,
        flush: bool,
        reply: ReplyEmpty,
    },
    FSync {
        ino: Inode,
        fh: FileHandle,
        datasync: bool,
        reply: ReplyEmpty,
    },
    OpenDir {
        ino: Inode,
        flags: i32,
        reply: fuser::ReplyOpen,
    },
    ReadDirPlus {
        ino: Inode,
        fh: FileHandle,
        offset: i64,
        reply: fuser::ReplyDirectoryPlus,
    },
    ReleaseDir {
        ino: Inode,
        fh: FileHandle,
        flags: i32,
        reply: ReplyEmpty,
    },
    FSyncDir {
        ino: Inode,
        fh: FileHandle,
        datasync: bool,
        reply: ReplyEmpty,
    },
    SetXattr {
        ino: Inode,
        name: OsString,
        value: Vec<u8>,
        flags: i32,
        position: u32,
        reply: ReplyEmpty,
    },
    GetXattr {
        ino: Inode,
        name: OsString,
        size: u32,
        reply: ReplyXattr,
    },
    ListXattr {
        ino: Inode,
        size: u32,
        reply: ReplyXattr,
    },
    RemoveXattr {
        ino: Inode,
        name: OsString,
        reply: ReplyEmpty,
    },
    Access {
        ino: Inode,
        mask: i32,
        reply: ReplyEmpty,
    },
    Create {
        parent: Inode,
        name: OsString,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: ReplyCreate,
    },
    GetLk {
        ino: Inode,
        fh: FileHandle,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        reply: ReplyLock,
    },
    SetLk {
        ino: Inode,
        fh: FileHandle,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
        reply: ReplyEmpty,
    },
    Bmap {
        ino: Inode,
        blocksize: u32,
        idx: u64,
        reply: ReplyBmap,
    },
    IoCtl {
        ino: Inode,
        fh: FileHandle,
        flags: u32,
        cmd: u32,
        in_data: Vec<u8>,
        out_size: u32,
        reply: fuser::ReplyIoctl,
    },
    Fallocate {
        ino: Inode,
        fh: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    },
    Lseek {
        ino: Inode,
        fh: FileHandle,
        offset: i64,
        whence: i32,
        reply: fuser::ReplyLseek,
    },
    CopyFileRange {
        ino_in: Inode,
        fh_in: FileHandle,
        offset_in: i64,
        ino_out: Inode,
        fh_out: FileHandle,
        offset_out: i64,
        len: u64,
        flags: u32,
        reply: ReplyWrite,
    },
}

impl FilesystemCommand {
    #[allow(unused)]
    pub fn name(&self) -> &'static str {
        match self {
            FilesystemCommand::HydrateChangeSets => "hydrate_change_sets",
            FilesystemCommand::GetAttr { .. } => "getattr",
            FilesystemCommand::ReadDir { .. } => "readdir",
            FilesystemCommand::Lookup { .. } => "lookup",
            FilesystemCommand::Forget { .. } => "forget",
            FilesystemCommand::SetAttr { .. } => "setattr",
            FilesystemCommand::ReadLink { .. } => "readlink",
            FilesystemCommand::MkNod { .. } => "mknod",
            FilesystemCommand::MkDir { .. } => "mkdir",
            FilesystemCommand::Unlink { .. } => "unlink",
            FilesystemCommand::RmDir { .. } => "rmdir",
            FilesystemCommand::SymLink { .. } => "symlink",
            FilesystemCommand::Rename { .. } => "rename",
            FilesystemCommand::Link { .. } => "link",
            FilesystemCommand::Open { .. } => "open",
            FilesystemCommand::Read { .. } => "read",
            FilesystemCommand::Write { .. } => "write",
            FilesystemCommand::Flush { .. } => "flush",
            FilesystemCommand::Release { .. } => "release",
            FilesystemCommand::FSync { .. } => "fsync",
            FilesystemCommand::OpenDir { .. } => "opendir",
            FilesystemCommand::ReadDirPlus { .. } => "readdirplus",
            FilesystemCommand::ReleaseDir { .. } => "releasedir",
            FilesystemCommand::FSyncDir { .. } => "fsyncdir",
            FilesystemCommand::SetXattr { .. } => "setxattr",
            FilesystemCommand::GetXattr { .. } => "getxattr",
            FilesystemCommand::ListXattr { .. } => "listxattr",
            FilesystemCommand::RemoveXattr { .. } => "removexattr",
            FilesystemCommand::Access { .. } => "access",
            FilesystemCommand::Create { .. } => "create",
            FilesystemCommand::GetLk { .. } => "getlk",
            FilesystemCommand::SetLk { .. } => "setlk",
            FilesystemCommand::Bmap { .. } => "bmap",
            FilesystemCommand::IoCtl { .. } => "ioctl",
            FilesystemCommand::Fallocate { .. } => "fallocate",
            FilesystemCommand::Lseek { .. } => "lseek",
            FilesystemCommand::CopyFileRange { .. } => "copy_file_range",
        }
    }

    pub fn error(self, errno: c_int) {
        match self {
            FilesystemCommand::HydrateChangeSets => {}
            FilesystemCommand::GetAttr { reply, .. } => reply.error(errno),
            FilesystemCommand::ReadDir { reply, .. } => reply.error(errno),
            FilesystemCommand::Lookup { reply, .. } => reply.error(errno),
            FilesystemCommand::Forget { .. } => {}
            FilesystemCommand::SetAttr { reply, .. } => reply.error(errno),
            FilesystemCommand::ReadLink { reply, .. } => reply.error(errno),
            FilesystemCommand::MkNod { reply, .. } => reply.error(errno),
            FilesystemCommand::MkDir { reply, .. } => reply.error(errno),
            FilesystemCommand::Unlink { reply, .. } => reply.error(errno),
            FilesystemCommand::RmDir { reply, .. } => reply.error(errno),
            FilesystemCommand::SymLink { reply, .. } => reply.error(errno),
            FilesystemCommand::Rename { reply, .. } => reply.error(errno),
            FilesystemCommand::Link { reply, .. } => reply.error(errno),
            FilesystemCommand::Open { reply, .. } => reply.error(errno),
            FilesystemCommand::Read { reply, .. } => reply.error(errno),
            FilesystemCommand::Write { reply, .. } => reply.error(errno),
            FilesystemCommand::Flush { reply, .. } => reply.error(errno),
            FilesystemCommand::Release { reply, .. } => reply.error(errno),
            FilesystemCommand::FSync { reply, .. } => reply.error(errno),
            FilesystemCommand::OpenDir { reply, .. } => reply.error(errno),
            FilesystemCommand::ReadDirPlus { reply, .. } => reply.error(errno),
            FilesystemCommand::ReleaseDir { reply, .. } => reply.error(errno),
            FilesystemCommand::FSyncDir { reply, .. } => reply.error(errno),
            FilesystemCommand::SetXattr { reply, .. } => reply.error(errno),
            FilesystemCommand::GetXattr { reply, .. } => reply.error(errno),
            FilesystemCommand::ListXattr { reply, .. } => reply.error(errno),
            FilesystemCommand::RemoveXattr { reply, .. } => reply.error(errno),
            FilesystemCommand::Access { reply, .. } => reply.error(errno),
            FilesystemCommand::Create { reply, .. } => reply.error(errno),
            FilesystemCommand::GetLk { reply, .. } => reply.error(errno),
            FilesystemCommand::SetLk { reply, .. } => reply.error(errno),
            FilesystemCommand::Bmap { reply, .. } => reply.error(errno),
            FilesystemCommand::IoCtl { reply, .. } => reply.error(errno),
            FilesystemCommand::Fallocate { reply, .. } => reply.error(errno),
            FilesystemCommand::Lseek { reply, .. } => reply.error(errno),
            FilesystemCommand::CopyFileRange { reply, .. } => reply.error(errno),
        }
    }
}
