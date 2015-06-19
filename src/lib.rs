#![feature(associated_types)]

extern crate libc;

use std::slice::Iter;
use std::collections::{hash_map, HashMap};
use std::rand::{thread_rng, Rng};
use std::mem::transmute;
use std::slice;
use std::slice::bytes::copy_memory;
use libc::{c_void, c_uchar, c_int, c_uint};

const MARKOV_ORDER: uint = 9;

#[derive(Show, Copy, Clone, PartialEq, Eq, Hash)]
struct MarkovKey([u8; MARKOV_ORDER]);

impl MarkovKey {
    pub fn new() -> MarkovKey {
        MarkovKey([b'\x00'; MARKOV_ORDER])
    }

    pub fn next(&self, next: u8) -> MarkovKey {
        let MarkovKey(mut data) = *self;
        let last_elem = data.len() - 1;
        for idx in 0..last_elem {
            data[idx] = data[idx + 1];
        }
        data[last_elem] = next;
        MarkovKey(data)
    }
}

trait AsMarkovIter {
    fn as_markov_iter<'a>(&'a self) -> MarkovIter<'a>;
}

impl AsMarkovIter for [u8] {
    fn as_markov_iter<'a>(&'a self) -> MarkovIter<'a> {
        MarkovIter {
            cur_key: MarkovKey::new(),
            source: self.iter(),
            finished: false,
        }
    }
}

impl AsMarkovIter for str {
    fn as_markov_iter<'a>(&'a self) -> MarkovIter<'a> {
        self.as_bytes().as_markov_iter()
    }
}

struct MarkovIter<'a> {
    cur_key: MarkovKey,
    source: Iter<'a, u8>,
    finished: bool,
}

impl<'a> Iterator for MarkovIter<'a> {
    type Item = (MarkovKey, u8);
    fn next(&mut self) -> Option<(MarkovKey, u8)> {
        if self.finished {
            return None;
        }
        match self.source.next() {
            Some(&chr) => {
                let emit_key = self.cur_key;
                self.cur_key = self.cur_key.next(chr);
                Some((emit_key, chr))
            },
            None => {
                self.finished = true;
                Some((self.cur_key, b'\0'))
            }
        }
    }
}

#[derive(Show)]
struct MarkovValue(u32, Vec<(u32, u8)>);

impl MarkovValue {
    pub fn from_char(val: u8) -> MarkovValue {
        MarkovValue(1, vec![(1, val)])
    }

    pub fn add(&mut self, val: u8) {
        let MarkovValue(ref mut count, ref mut vec) = *self;
        *count += 1;
        for &mut (ref mut prob, candidate) in vec.iter_mut() {
            if candidate == val {
                *prob += 1;
                return;
            }
        }
        vec.push((1, val));
    }

    pub fn pick<R>(&self, rng: &mut R) -> u8 where R: Rng {
        let &MarkovValue(count, ref vec) = self;
        let mut target = rng.gen_range(0, count);
        for &(sub, chr) in vec.iter() {
            if target < sub {
                return chr;
            }
            target -= sub;
        }
        unreachable!();
    }
}

#[derive(Show)]
pub struct MarkovGenerator {
    table: HashMap<MarkovKey, MarkovValue>,
}

impl MarkovGenerator {
    pub fn new() -> MarkovGenerator {
        MarkovGenerator {
            table: HashMap::new(),
        }
    }

    pub fn learn(&mut self, source: &str) {
        if source.len() < 20 {
            return;
        }
        for (key, next) in source.as_markov_iter() {
            match self.table.entry(key) {
                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().add(next);
                },
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(MarkovValue::from_char(next));
                }
            }
        }
    }

    pub fn speak(&self) -> Result<String, ()> {
        self.speak_from_key(MarkovKey::new())
    }

    fn reply(&self, message: &str) -> Result<String, ()> {
        let keys: Vec<_> = message.as_markov_iter().collect();
        for (key, _) in keys.into_iter().rev() {
            println!("Attempt key = {:?}", key);
            match self.speak_from_key(key) {
                Ok(res) => return Ok(res),
                Err(_) => ()
            }
        }
        Err(())
    }

    fn attempt_speak_from_key(&self, key: MarkovKey) -> Result<String, ()> {
        let mut key = key;
        let mut output = Vec::new();
        let mut rng = thread_rng();

        {
            let MarkovKey(ref chars) = key;
            for &chr in chars.iter() {
                if chr != b'\0' {
                    output.push(chr);
                }
            }
        }

        loop {
            match self.table.get(&key) {
                Some(value) => {
                    let next_char = value.pick(&mut rng);
                    if next_char == b'\0' {
                        break;
                    }
                    output.push(next_char);
                    key = key.next(next_char);
                }
                None => break,
            }
        }
        match String::from_utf8(output) {
            Ok(strval) => Ok(strval),
            Err(_) => Err(()),
        }
    }

    fn speak_from_key(&self, key: MarkovKey) -> Result<String, ()> {
        for _ in 0..12 {
            match self.attempt_speak_from_key(key) {
                Ok(res) => return Ok(res),
                Err(_) => ()
            }
        }
        Err(())
    }
}

impl Drop for MarkovGenerator {
    fn drop(&mut self) {
        println!("Dropping MarkovGenerator");
    }
}

#[no_mangle]
pub extern "C" fn markov_alloc() -> *mut c_void {
    let markov_generator = Box::new(MarkovGenerator::new());
    unsafe { transmute(markov_generator) }
}

#[no_mangle]
pub extern "C" fn markov_dealloc(ptr: *mut c_void) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let _: Box<MarkovGenerator> = unsafe { transmute(ptr) };
    return 0;
}

#[no_mangle]
pub extern "C" fn markov_reply(
    ptr: *mut c_void,
    ibuf: *const c_uchar, ilen: c_int,
    obuf: *mut c_uchar, olen: c_int,
) -> c_int {
    if ptr.is_null() || ibuf.is_null() || obuf.is_null() {
        return -1;
    }

    let input_buf = unsafe { slice::from_raw_buf(&ibuf, ilen as uint) };
    let mut owned_buf = Vec::new();
    owned_buf.push_all(input_buf.as_slice());
    let to_learn = match String::from_utf8(owned_buf) {
        Ok(string) => string,
        Err(_) => return -2,
    };

    let gen: Box<MarkovGenerator> = unsafe { transmute(ptr) };
    let phrase_res = gen.reply(to_learn.as_slice());
    let _: *mut c_void = unsafe { transmute(gen) };

    let phrase = match phrase_res {
        Ok(response) => response,
        Err(()) => return -3,
    };

    
    let mut output_buf = unsafe { slice::from_raw_mut_buf(&obuf, olen as uint) };
    if phrase.as_bytes().len() < output_buf.len() {
        copy_memory(output_buf.as_mut_slice(), phrase.as_bytes());
        phrase.as_bytes().len() as c_int
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn markov_learn(ptr: *const c_void, buf: *const c_uchar, len: c_int) -> c_int {
    if ptr.is_null() {
        return -1;
    }

    let input_buf = unsafe { slice::from_raw_buf(&buf, len as uint) };

    let mut owned_buf = Vec::new();
    owned_buf.push_all(input_buf);

    let to_learn = match String::from_utf8(owned_buf) {
        Ok(string) => string,
        Err(_) => return -2,
    };

    let mut gen: Box<MarkovGenerator> = unsafe { transmute(ptr) };
    gen.learn(to_learn.as_slice());
    let _: *mut c_void = unsafe { transmute(gen) };

    0
}


#[no_mangle]
pub extern "C" fn markov_speak(ptr: *mut c_void, buf: *mut c_uchar, len: c_uint) -> c_int {
    if ptr.is_null() {
        return 0;
    }
    let mut output_buf = unsafe { slice::from_raw_mut_buf(&buf, len as uint) };
    
    let gen: Box<MarkovGenerator> = unsafe { transmute(ptr) };
    let phrase_res = gen.speak();
    let _: *mut c_void = unsafe { transmute(gen) };

    let phrase = match phrase_res {
        Ok(phrase) => phrase,
        Err(()) => return -1,
    };

    if phrase.as_bytes().len() < output_buf.len() {
        copy_memory(output_buf.as_mut_slice(), phrase.as_bytes());
        phrase.as_bytes().len() as c_int
    } else {
        0
    }
}

