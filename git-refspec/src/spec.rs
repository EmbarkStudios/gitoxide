use bstr::{BStr, ByteSlice};

use crate::{
    instruction::{Fetch, Push},
    parse::Operation,
    types::Mode,
    Instruction, RefSpec, RefSpecRef,
};

/// Conversion. Use the [RefSpecRef][RefSpec::to_ref()] type for more usage options.
impl RefSpec {
    /// Return ourselves as reference type.
    pub fn to_ref(&self) -> RefSpecRef<'_> {
        RefSpecRef {
            mode: self.mode,
            op: self.op,
            src: self.src.as_ref().map(|b| b.as_ref()),
            dst: self.dst.as_ref().map(|b| b.as_ref()),
        }
    }
}

mod impls {
    use std::{
        cmp::Ordering,
        hash::{Hash, Hasher},
    };

    use crate::{RefSpec, RefSpecRef};

    impl From<RefSpecRef<'_>> for RefSpec {
        fn from(v: RefSpecRef<'_>) -> Self {
            v.to_owned()
        }
    }

    impl Hash for RefSpec {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.to_ref().hash(state)
        }
    }

    impl Hash for RefSpecRef<'_> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.instruction().hash(state)
        }
    }

    impl PartialEq for RefSpec {
        fn eq(&self, other: &Self) -> bool {
            self.to_ref().eq(&other.to_ref())
        }
    }

    impl PartialEq for RefSpecRef<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.instruction().eq(&other.instruction())
        }
    }

    impl PartialOrd for RefSpecRef<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            self.instruction().partial_cmp(&other.instruction())
        }
    }

    impl PartialOrd for RefSpec {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            self.to_ref().partial_cmp(&other.to_ref())
        }
    }

    impl Ord for RefSpecRef<'_> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.instruction().cmp(&other.instruction())
        }
    }

    impl Ord for RefSpec {
        fn cmp(&self, other: &Self) -> Ordering {
            self.to_ref().cmp(&other.to_ref())
        }
    }
}

/// Access
impl<'a> RefSpecRef<'a> {
    /// Return the left-hand side of the spec, typically the source.
    /// It takes many different forms so don't rely on this being a ref name.
    ///
    /// It's not present in case of deletions.
    pub fn source(&self) -> Option<&BStr> {
        self.src
    }

    /// Return the right-hand side of the spec, typically the destination.
    /// It takes many different forms so don't rely on this being a ref name.
    ///
    /// It's not present in case of source-only specs.
    pub fn destination(&self) -> Option<&BStr> {
        self.dst
    }

    /// Always returns the remote side, whose actual side in the refspec depends on how it was parsed.
    pub fn remote(&self) -> Option<&BStr> {
        match self.op {
            Operation::Push => self.dst,
            Operation::Fetch => self.src,
        }
    }

    /// Always returns the local side, whose actual side in the refspec depends on how it was parsed.
    pub fn local(&self) -> Option<&BStr> {
        match self.op {
            Operation::Push => self.src,
            Operation::Fetch => self.dst,
        }
    }

    /// Derive the prefix from the `source` side of this spec, if possible.
    ///
    /// This means it starts with `refs/`. Note that it won't contain more than two components, like `refs/heads/`
    pub fn prefix(&self) -> Option<&BStr> {
        let source = self.source()?;
        let suffix = source.strip_prefix(b"refs/")?;
        let slash_pos = suffix.find_byte(b'/')?;
        let prefix = source[..="refs/".len() + slash_pos].as_bstr();
        (!prefix.contains(&b'*')).then(|| prefix)
    }

    /// Transform the state of the refspec into an instruction making clear what to do with it.
    pub fn instruction(&self) -> Instruction<'a> {
        match self.op {
            Operation::Fetch => match (self.mode, self.src, self.dst) {
                (Mode::Normal | Mode::Force, Some(src), None) => Instruction::Fetch(Fetch::Only { src }),
                (Mode::Normal | Mode::Force, Some(src), Some(dst)) => Instruction::Fetch(Fetch::AndUpdate {
                    src,
                    dst,
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Negative, Some(src), None) => Instruction::Fetch(Fetch::Exclude { src }),
                (mode, src, dest) => {
                    unreachable!(
                        "BUG: fetch instructions with {:?} {:?} {:?} are not possible",
                        mode, src, dest
                    )
                }
            },
            Operation::Push => match (self.mode, self.src, self.dst) {
                (Mode::Normal | Mode::Force, Some(src), None) => Instruction::Push(Push::Matching {
                    src,
                    dst: src,
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Normal | Mode::Force, None, Some(dst)) => {
                    Instruction::Push(Push::Delete { ref_or_pattern: dst })
                }
                (Mode::Normal | Mode::Force, None, None) => Instruction::Push(Push::AllMatchingBranches {
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Normal | Mode::Force, Some(src), Some(dst)) => Instruction::Push(Push::Matching {
                    src,
                    dst,
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (mode, src, dest) => {
                    unreachable!(
                        "BUG: push instructions with {:?} {:?} {:?} are not possible",
                        mode, src, dest
                    )
                }
            },
        }
    }
}

/// Conversion
impl RefSpecRef<'_> {
    /// Convert this ref into a standalone, owned copy.
    pub fn to_owned(&self) -> RefSpec {
        RefSpec {
            mode: self.mode,
            op: self.op,
            src: self.src.map(ToOwned::to_owned),
            dst: self.dst.map(ToOwned::to_owned),
        }
    }
}
