// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::hash::{BuildHasherDefault, Hasher};

use std::ptr::NonNull;

pub(crate) struct Entry {
    pub(crate) string: Box<str>,
    pub(crate) hash: u32,
}

#[test]
fn entry_alignment_is_sufficient() {
    // Addresses are a multiples of this,
    // and therefore have have TAG_MASK bits unset, available for tagging.
    const ENTRY_ALIGNMENT: usize = 4;
    assert!(std::mem::align_of::<Entry>() >= ENTRY_ALIGNMENT);
}

use dashmap::DashMap;

pub(crate) struct Set(DashMap<u32, Box<Entry>, BuildHasherDefault<IdentityHasher>>);

#[derive(Default)]
pub struct IdentityHasher {
    hash: u64,
}

impl Hasher for IdentityHasher {
    fn write(&mut self, _: &[u8]) {
        panic!("Invalid use of IdentityHasher")
    }

    fn write_u32(&mut self, n: u32) {
        self.hash = n as u64
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub(crate) static DYNAMIC_SET: Lazy<Set> = Lazy::new(|| Set(DashMap::default()));

impl Set {
    pub(crate) fn insert(&self, string: Cow<str>, hash: u32) -> NonNull<Entry> {
        match self.0.entry(hash) {
            dashmap::mapref::entry::Entry::Occupied(s) => NonNull::from(&**s.get()),
            dashmap::mapref::entry::Entry::Vacant(v) => {
                let s = string.to_string().into_boxed_str();
                let entry = Box::new(Entry { string: s, hash });
                let ptr = NonNull::from(&*entry);
                v.insert(entry);
                ptr
            }
        }
    }
}
