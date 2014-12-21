extern crate libc;

use std::collections::{hash_map, HashMap};
use std::str::Chars;
use std::rand::{task_rng, Rng};
use std::mem::transmute;
use std::c_vec::CVec;
use std::slice::bytes::copy_memory;
use libc::{c_void, c_uchar, c_int, c_uint};

const MARKOV_ORDER: uint = 9;

#[deriving(Show, Copy, PartialEq, Eq, Hash)]
struct MarkovKey([char, ..MARKOV_ORDER]);

impl MarkovKey {
    pub fn new() -> MarkovKey {
        MarkovKey(['\x00', ..MARKOV_ORDER])
    }

    pub fn next(&self, next: char) -> MarkovKey {
        let MarkovKey(mut data) = *self;
        let last_elem = data.len() - 1;
        for idx in range(0, last_elem) {
            data[idx] = data[idx + 1];
        }
        data[last_elem] = next;
        MarkovKey(data)
    }
}

trait AsMarkovIter for Sized? {
    fn as_markov_iter<'a>(&'a self) -> MarkovIter<'a>;
}

impl AsMarkovIter for str {
    fn as_markov_iter<'a>(&'a self) -> MarkovIter<'a> {
        MarkovIter {
            cur_key: MarkovKey::new(),
            source: self.chars()
        }
    }
}

struct MarkovIter<'a> {
    cur_key: MarkovKey,
    source: Chars<'a>,
}

impl<'a> Iterator<(MarkovKey, char)> for MarkovIter<'a> {
    fn next(&mut self) -> Option<(MarkovKey, char)> {
        match self.source.next() {
            Some(chr) => {
                let emit_key = self.cur_key;
                self.cur_key = self.cur_key.next(chr);
                Some((emit_key, chr))
            },
            None => None
        }
    }
}

#[deriving(Show)]
struct MarkovValue(u32, Vec<(u32, char)>);

impl MarkovValue {
    pub fn from_char(val: char) -> MarkovValue {
        MarkovValue(1, vec![(1, val)])
    }

    pub fn add(&mut self, val: char) {
        let &MarkovValue(ref mut count, ref mut vec) = self;
        *count += 1;
        for &(ref mut prob, candidate) in vec.iter_mut() {
            if candidate == val {
                *prob += 1;
                return;
            }
        }
        vec.push((1, val));
    }

    pub fn pick<R>(&self, rng: &mut R) -> char where R: Rng {
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

#[deriving(Show)]
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
                    entry.set(MarkovValue::from_char(next));
                }
            }
        }
    }

    pub fn speak(&self) -> String {
        self.speak_from_key(MarkovKey::new())
    }

    fn speak_from_key(&self, key: MarkovKey) -> String {
        let mut key = key;
        let mut output = String::new();
        let mut rng = task_rng();

        {
            let MarkovKey(ref chars) = key;
            for &chr in chars.iter() {
                if chr != '\0' {
                    output.push(chr);
                }
            }
        }

        loop {
            match self.table.get(&key) {
                Some(value) => {
                    let next_char = value.pick(&mut rng);
                    if next_char != '\0' {
                        output.push(next_char);
                    }
                    key = key.next(next_char);
                }
                None => break
            }

            // FIXME: we need an end-of-sentence marker.
            if 4096 < output.len() {
                break;
            }
        }
        output
    }    
}

#[no_mangle]
pub extern "C" fn markov_alloc() -> *mut c_void {
    let markov_generator = box MarkovGenerator::new();
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
pub extern "C" fn markov_learn(ptr: *mut c_void, buf: *mut c_uchar, len: c_int) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let mut gen: Box<MarkovGenerator> = unsafe { transmute(ptr) };
    let input_buf = unsafe { CVec::new(buf, len as uint) };

    let mut owned_buf = Vec::new();
    owned_buf.push_all(input_buf.as_slice());

    let to_learn = match String::from_utf8(owned_buf) {
        Ok(string) => string,
        Err(_) => return -2,
    };
    gen.learn(to_learn.as_slice());

    // Prevent drop
    let _: *mut c_void = unsafe { transmute(gen) };

    0
}


#[no_mangle]
pub extern "C" fn markov_speak(ptr: *mut c_void, buf: *mut c_uchar, len: c_uint) -> c_int {
    if ptr.is_null() {
        return 0;
    }
    let mut output_buf = unsafe { CVec::new(buf, len as uint) };
    let gen: Box<MarkovGenerator> = unsafe { transmute(ptr) };
    let phrase = gen.speak();

    // Prevent drop
    let _: *mut c_void = unsafe { transmute(gen) };

    if phrase.as_bytes().len() < output_buf.len() {
        copy_memory(output_buf.as_mut_slice(), phrase.as_bytes());
        phrase.as_bytes().len() as c_int
    } else {
        0
    }
}
