use std::{
    error::Error,
    fmt::{Debug, Display},
};

pub struct Errno {
    raw_errno: i32,
}

impl Errno {
    pub fn new(errno: i32) -> Self {
        Self { raw_errno: errno }
    }
    pub fn errno() -> Self {
        Self::new(unsafe { *libc::__errno_location() } )
    }
}

impl<E: Into<i32>> From<E> for Errno {
    fn from(value: E) -> Self {
        Self {
            raw_errno: value.into(),
        }
    }
}

impl Display for Errno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = unsafe {
            let cstr = libc::strerror(self.raw_errno);
            let len = libc::strlen(cstr);

            let mut buffer = Vec::with_capacity(len);
            buffer.set_len(len);

            libc::strncpy(buffer.as_mut_ptr() as *mut i8, cstr, len);

            String::from_utf8(buffer)
        };
        let result = if let Ok(str) = str {
            f.write_str(&str)
        } else {
            f.write_str(&format!("Unknown error {}", self.raw_errno))
        };

        result
    }
}

impl Debug for Errno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Errno")
            .field("errno", &self.raw_errno)
            .finish()
    }
}

impl Error for Errno {}

#[cfg(test)]
mod test {
    use super::Errno;

    #[test]
    fn test_errno() {
        assert_eq!(
            Errno::new(libc::ENOENT).to_string(),
            "No such file or directory"
        );
        assert_eq!(Errno::new(0).to_string(), "Success");
    }
}
