use std::fmt::{self, Debug, Write as _};

/// This cannot be used on types with alignment == 1.
///
/// ```rust
/// use constant_size_dfs::tagged_ptr::TaggedPtr;
/// let mut v: u16 = 1;
/// let ptr = TaggedPtr::from_untagged(&mut v);
/// ```
///
/// ```compile_fail
/// use constant_size_dfs::tagged_ptr::TaggedPtr;
/// let mut v: u8 = 1;
/// let ptr = TaggedPtr::from_untagged(&mut v);
/// ```
pub struct TaggedPtr<T>(*mut T);
const SEEN_BIT: usize = 1;

impl<T: Debug> Debug for TaggedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.as_untagged();
        let flag = self.0 as usize & SEEN_BIT;
        write!(f, "<0x{:0x}|{}>", ptr as usize, flag)?;
        if let Some(node) = unsafe { ptr.as_ref() } {
            f.write_char(' ')?;
            node.fmt(f)?;
        }
        Ok(())
    }
}

impl<T> Clone for TaggedPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TaggedPtr<T> {}

impl<T> TaggedPtr<T> {
    const ALIGN_OK: () = assert!(align_of::<T>() > 1);

    pub const fn from_untagged(ptr: *mut T) -> Self {
        let () = Self::ALIGN_OK;
        Self(ptr)
    }

    pub fn as_untagged(self) -> *mut T {
        let addr = self.0 as usize & !SEEN_BIT;
        addr as _
    }

    pub fn is_seen(self) -> bool {
        self.0 as usize & SEEN_BIT == SEEN_BIT
    }

    pub fn seen(self) -> Self {
        let addr = self.0 as usize | SEEN_BIT;
        Self(addr as _)
    }

    pub fn unseen(self) -> Self {
        let addr = self.0 as usize & !SEEN_BIT;
        Self(addr as _)
    }
}
