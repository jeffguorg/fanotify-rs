pub use libc::{
    FAN_ACCESS, FAN_ACCESS_PERM, FAN_ALLOW, FAN_ATTRIB, FAN_AUDIT, FAN_CLASS_CONTENT,
    FAN_CLASS_NOTIF, FAN_CLASS_PRE_CONTENT, FAN_CLOEXEC, FAN_CLOSE, FAN_CLOSE_NOWRITE,
    FAN_CLOSE_WRITE, FAN_CREATE, FAN_DELETE, FAN_DELETE_SELF, FAN_DENY, FAN_ENABLE_AUDIT,
    FAN_EPIDFD, FAN_EVENT_INFO_TYPE_DFID, FAN_EVENT_INFO_TYPE_DFID_NAME, FAN_EVENT_INFO_TYPE_ERROR,
    FAN_EVENT_INFO_TYPE_FID, FAN_EVENT_INFO_TYPE_NEW_DFID_NAME, FAN_EVENT_INFO_TYPE_OLD_DFID_NAME,
    FAN_EVENT_INFO_TYPE_PIDFD, FAN_EVENT_ON_CHILD, FAN_FS_ERROR, FAN_INFO, FAN_MARK_ADD,
    FAN_MARK_DONT_FOLLOW, FAN_MARK_EVICTABLE, FAN_MARK_FILESYSTEM, FAN_MARK_FLUSH, FAN_MARK_IGNORE,
    FAN_MARK_IGNORE_SURV, FAN_MARK_IGNORED_MASK, FAN_MARK_IGNORED_SURV_MODIFY, FAN_MARK_INODE,
    FAN_MARK_MOUNT, FAN_MARK_ONLYDIR, FAN_MARK_REMOVE, FAN_MODIFY, FAN_MOVE, FAN_MOVE_SELF,
    FAN_MOVED_FROM, FAN_MOVED_TO, FAN_NOFD, FAN_NONBLOCK, FAN_NOPIDFD, FAN_ONDIR, FAN_OPEN,
    FAN_OPEN_EXEC, FAN_OPEN_EXEC_PERM, FAN_OPEN_PERM, FAN_Q_OVERFLOW, FAN_RENAME,
    FAN_REPORT_DFID_NAME, FAN_REPORT_DFID_NAME_TARGET, FAN_REPORT_DIR_FID, FAN_REPORT_FID,
    FAN_REPORT_NAME, FAN_REPORT_PIDFD, FAN_REPORT_TARGET_FID, FAN_REPORT_TID,
    FAN_RESPONSE_INFO_AUDIT_RULE, FAN_RESPONSE_INFO_NONE, FAN_UNLIMITED_MARKS, FAN_UNLIMITED_QUEUE,
    FANOTIFY_METADATA_VERSION,
};

pub use libc::{
    O_APPEND, O_CLOEXEC, O_DSYNC, O_LARGEFILE, O_NOATIME, O_NONBLOCK, O_RDONLY, O_RDWR, O_SYNC,
    O_WRONLY,
};

// NOTE: the definitions is handwritten in 2025-04-03, on Debian trixie(testing) 6.12.20-1 x86_64
// it may need updating for future kernel updates, and pls update this comment for maintainence
fa_bitflags! {
    pub struct InitFlags: u32 {
        /*
        one of three classes:
        - notification-only: get notified when file or directory is accessed
        - content-access: get notified and also check permission when content is ready. usually used by security softwares.
        - pre-content-access: get notified and also check permission BEFORE content is ready. usually used by storage managers.
         */
        FAN_CLASS_NOTIF;
        FAN_CLASS_CONTENT;
        FAN_CLASS_PRE_CONTENT;

        // additional flags
        FAN_CLOEXEC;
        FAN_NONBLOCK;
        FAN_UNLIMITED_QUEUE;
        FAN_UNLIMITED_MARKS;
        FAN_ENABLE_AUDIT; // Linux 4.15

        FAN_REPORT_TID; // Linux 4.20
        FAN_REPORT_FID; // Linux 5.1
        FAN_REPORT_DIR_FID; // Linux 5.9
        FAN_REPORT_NAME; // Linux 5.9
        FAN_REPORT_DFID_NAME; // Linux 5.9, FAN_REPORT_DIR_FID|FAN_REPORT_NAME
        FAN_REPORT_TARGET_FID; // Linux 5.17 / 5.15.154 / 5.10.220
        FAN_REPORT_DFID_NAME_TARGET; // Linux 5.17 / 5.15.154 / 5.10.220, FAN_REPORT_DFID_NAME|FAN_REPORT_FID|FAN_REPORT_TARGET_FID
        FAN_REPORT_PIDFD; // Linux 5.15 / 5.10.220
    }

    pub struct EventFFlags: ~u32 {
        O_RDONLY;
        O_WRONLY;
        O_RDWR;

        O_LARGEFILE; // file size limit 2G+
        O_CLOEXEC; // Linux 3.18

        O_APPEND;
        O_DSYNC;
        O_NOATIME;
        O_NONBLOCK;
        O_SYNC;
    }

    pub struct MarkFlags: u32 {
        // operation, choose exactly one
        FAN_MARK_ADD;
        FAN_MARK_REMOVE;
        FAN_MARK_FLUSH;

        // additional flags
        FAN_MARK_DONT_FOLLOW;
        FAN_MARK_ONLYDIR;
        FAN_MARK_MOUNT;
        FAN_MARK_FILESYSTEM; // Linux 4.20
        FAN_MARK_IGNORED_MASK;
        FAN_MARK_IGNORE; // Linux 6.0 / 5.15.154 / 5.10.220
        FAN_MARK_IGNORED_SURV_MODIFY;
        FAN_MARK_IGNORE_SURV; // Linux 6.0 / 5.15.154 / 5.10.220, FAN_MARK_IGNORE|FAN_MARK_IGNORED_SURV_MODIFY
        FAN_MARK_EVICTABLE; // Linux 5.19 / 5.15.154 / 5.10.220
    }

    pub struct MaskFlags: u64 {
        FAN_ACCESS;
        FAN_MODIFY;
        FAN_CLOSE_WRITE;
        FAN_CLOSE_NOWRITE;
        FAN_CLOSE; // FAN_CLOSE_WRITE|FAN_CLOSE_NOWRITE
        FAN_OPEN;
        FAN_OPEN_EXEC; // Linux 5.0
        FAN_ATTRIB; // Linux 5.1
        FAN_CREATE; // Linux 5.1
        FAN_DELETE; // Linux 5.1
        FAN_DELETE_SELF; // Linux 5.1
        FAN_FS_ERROR; // Linux 5.16 / 5.15.154 / 5.10.220
        FAN_MOVED_FROM; // Linux 5.1
        FAN_MOVED_TO; // Linux 5.1
        FAN_MOVE; // Linux 5.1, FAN_MOVED_FROM|FAN_MOVED_TO
        FAN_RENAME; // Linux 5.17 / 5.15.154 / 5.10.220
        FAN_MOVE_SELF; // Linux 5.1

        // Permissions, need FAN_CLASS_CONTENT or FAN_CLASS_PRE_CONTENT on init
        FAN_OPEN_PERM;
        FAN_ACCESS_PERM;
        FAN_OPEN_EXEC_PERM; // Linux 5.0

        // Flags
        FAN_ONDIR; // enable events on directories
        FAN_EVENT_ON_CHILD; // enable events on direct
    }
}
