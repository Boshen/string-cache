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
use std::ptr::NonNull;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Mutex;

const NB_BUCKETS: usize = 1 << 12; // 4096
const BUCKET_MASK: u32 = (1 << 12) - 1;

pub(crate) struct Set {
    buckets: Box<[Mutex<Vec<Box<Entry>>>]>,
}

pub(crate) struct Entry {
    pub(crate) string: Box<str>,
    pub(crate) hash: u32,
    pub(crate) ref_count: AtomicIsize,
}

#[test]
fn entry_alignment_is_sufficient() {
    // Addresses are a multiples of this,
    // and therefore have have TAG_MASK bits unset, available for tagging.
    const ENTRY_ALIGNMENT: usize = 4;
    assert!(std::mem::align_of::<Entry>() >= ENTRY_ALIGNMENT);
}

pub(crate) static DYNAMIC_SET: Lazy<Set> = Lazy::new(|| {
    let buckets = (0..NB_BUCKETS).map(|_| Mutex::new(vec![])).collect();
    Set { buckets }
});

impl Set {
    pub(crate) fn insert(&self, string: Cow<str>, hash: u32) -> NonNull<Entry> {
        let bucket_index = (hash & BUCKET_MASK) as usize;
        let mut vec = self.buckets[bucket_index].lock().unwrap();
        if let Some(entry) = vec
            .iter_mut()
            .find(|e| e.hash == hash && *e.string == *string)
        {
            if entry.ref_count.fetch_add(1, SeqCst) > 0 {
                return NonNull::from(&mut **entry);
            }
            // Uh-oh. The pointer's reference count was zero, which means someone may try
            // to free it. (Naive attempts to defend against this, for example having the
            // destructor check to see whether the reference count is indeed zero, don't
            // work due to ABA.) Thus we need to temporarily add a duplicate string to the
            // list.
            entry.ref_count.fetch_sub(1, SeqCst);
        }

        let string = string.into_owned();
        let mut entry = Box::new(Entry {
            hash,
            ref_count: AtomicIsize::new(1),
            string: string.into_boxed_str(),
        });
        let ptr = NonNull::from(&mut *entry);
        vec.push(entry);
        ptr
    }

    pub(crate) fn remove(&self, ptr: *const Entry) {
        let bucket_index = {
            let value: &Entry = unsafe { &*ptr };
            debug_assert!(value.ref_count.load(SeqCst) == 0);
            (value.hash & BUCKET_MASK) as usize
        };

        let mut vec = self.buckets[bucket_index].lock().unwrap();

        vec.retain(|e| (&**e as *const Entry) != ptr);
    }
}
