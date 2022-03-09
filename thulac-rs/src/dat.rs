use std::{
    io::{BufRead, Error, ErrorKind, Read, Result, Seek, Write},
    iter::Peekable,
    mem::size_of,
    slice,
    str::Chars,
};

#[derive(Clone, Default)]
#[repr(C)]
struct DatEntry {
    base: i32,
    check: i32,
}

impl DatEntry {
    fn prev(&self) -> i32 {
        -self.base
    }
    fn set_prev(&mut self, prev: i32) {
        self.base = -prev;
    }
    fn next(&self) -> i32 {
        -self.check
    }
    fn set_next(&mut self, next: i32) {
        self.check = -next;
    }
    fn used(&self) -> bool {
        self.check >= 0
    }
}

pub struct Dat {
    entries: Vec<DatEntry>,
}

impl Dat {
    pub fn load<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let len = reader.stream_len()? as usize;
        if len % size_of::<DatEntry>() != 0 {
            return Err(Error::new(ErrorKind::Other, "file size unexpected"));
        }
        let mut entries = vec![DatEntry::default(); len / size_of::<DatEntry>()];
        reader.read_exact(unsafe {
            slice::from_raw_parts_mut(entries.as_mut_ptr() as *mut u8, len)
        })?;
        Ok(Self { entries })
    }
    pub fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(unsafe {
            slice::from_raw_parts(
                self.entries.as_ptr() as *const u8,
                self.entries.len() * size_of::<DatEntry>(),
            )
        })?;
        Ok(())
    }
    pub fn load_set_txt<R: BufRead>(reader: &mut R, insert_end: bool) -> Result<Self> {
        let mut words = reader.lines().collect::<Result<Vec<_>>>()?;
        if insert_end {
            words.iter_mut().for_each(|x| x.push('\0'));
        }
        let entries = words.iter().map(|x| (&x[..], 0)).collect::<Vec<_>>();
        Ok(Self::build(entries))
    }
    pub fn load_map_txt<R: BufRead>(reader: &mut R, insert_end: bool) -> Result<Self> {
        let mut words = reader
            .lines()
            .map(|x| {
                x.and_then(|x| {
                    x.rfind('\t')
                        .map(move |i| (x, i))
                        .ok_or_else(|| Error::new(ErrorKind::Other, "missing delimiter"))
                })
                .and_then(|(mut x, i)| {
                    x[i + 1..]
                        .parse::<i32>()
                        .map(move |v| {
                            x.drain(i..);
                            (x, v)
                        })
                        .map_err(|_| Error::new(ErrorKind::Other, "invalid value of entry"))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if insert_end {
            words.iter_mut().for_each(|x| x.0.push('\0'));
        }
        let entries = words.iter().map(|(x, v)| (&x[..], *v)).collect::<Vec<_>>();
        Ok(Self::build(entries))
    }
    pub fn root(&self) -> i32 {
        0
    }
    pub fn child(&self, parent: i32, offset: char) -> Option<i32> {
        let item = self.entries[parent as usize].base + offset as i32;
        if (item as usize) < self.entries.len() && self.entries[item as usize].check == parent {
            Some(item)
        } else {
            None
        }
    }
    pub fn and_child(&self, parent: Option<i32>, offset: char) -> Option<i32> {
        parent.and_then(move |x| self.child(x, offset))
    }
    pub fn descendant(&self, parent: i32, offset: &str) -> Option<i32> {
        offset
            .chars()
            .fold(Some(parent), |p, ch| self.and_child(p, ch))
    }
    pub fn base(&self, node: i32) -> i32 {
        self.entries[node as usize].base
    }
    /// any entry cannot be a prefix of anther entry. You can append a
    /// special character to each string to do this.
    pub fn build(mut map: Vec<(&str, i32)>) -> Self {
        map.sort_unstable_by_key(|x| x.0);
        'outer: for (prev, curr) in map
            .iter()
            .map(|x| x.0.as_bytes())
            .zip(map.iter().skip(1).map(|x| x.0.as_bytes()))
        {
            if prev.len() > curr.len() {
                continue;
            }
            for i in (0..prev.len()).rev() {
                if prev[i] != curr[i] {
                    continue 'outer;
                }
            }
            panic!("some entry is a prefix of or equals to another entry");
        }

        fn process(
            builder: &mut DatBuilder,
            map: &mut [(Peekable<Chars<'_>>, i32)],
            check: i32,
        ) -> i32 {
            let len = map.len();
            let first = map.first_mut().unwrap();
            if first.0.peek().is_none() {
                assert_eq!(len, 1);
                return first.1;
            }
            let mut last_start = 0;
            let mut last_offset = *first.0.peek().unwrap() as i32;
            let base_offset = last_offset;
            let mut offsets = Vec::new();
            let mut values = Vec::new();
            for i in 0..map.len() {
                let ch = map[i].0.next().unwrap() as i32;
                if ch != last_offset {
                    offsets.push(last_offset - base_offset);
                    values.push(last_start..i);
                    last_offset = ch;
                    last_start = i;
                }
            }
            offsets.push(last_offset - base_offset);
            values.push(last_start..map.len());
            let base = builder.alloc(&offsets);
            for (offset, value) in offsets.into_iter().zip(values.into_iter()) {
                let index = base + offset;
                let base = process(builder, &mut map[value], index);
                builder.set(index, DatEntry { base, check });
            }
            base - base_offset
        }

        let mut builder = DatBuilder::new();
        let mut map = map
            .into_iter()
            .map(|(string, value)| (string.chars().peekable(), value))
            .collect::<Vec<_>>();
        let base = process(&mut builder, &mut map, 0);
        builder.set(0, DatEntry { base, check: 0 });
        builder.cleanup()
    }
}

struct DatBuilder {
    dat: Vec<DatEntry>,
}

impl DatBuilder {
    fn new() -> Self {
        Self {
            dat: vec![
                DatEntry { base: 0, check: 0 }, // root
                DatEntry {
                    base: -1,
                    check: -1,
                }, // sentinel
            ],
        }
    }
    fn sentinel(&self) -> i32 {
        (self.dat.len() - 1) as i32
    }
    fn use_(&mut self, index: i32) {
        assert!(!self.dat[index as usize].used());
        let prev = self.dat[index as usize].prev();
        let next = self.dat[index as usize].next();
        self.dat[prev as usize].set_next(next);
        self.dat[next as usize].set_prev(prev);
        self.dat[index as usize] = DatEntry {
            base: 0,
            check: index,
        };
    }
    fn set(&mut self, index: i32, entry: DatEntry) {
        assert!(self.dat[index as usize].used());
        self.dat[index as usize] = entry;
    }
    fn extend(&mut self) {
        let old_size = self.dat.len();
        let new_size = 2 * self.dat.len();
        let old_sentinel = self.sentinel();
        let mut index = old_size as i32;
        self.dat.resize_with(new_size, || {
            let entry = DatEntry {
                base: -(index - 1),
                check: -(index + 1),
            };
            index += 1;
            entry
        });
        let new_sentinel = self.sentinel();
        let old_head = self.dat[old_sentinel as usize].next();
        self.dat[old_sentinel as usize].set_next(old_size as i32);
        self.dat[old_head as usize].set_prev(new_sentinel);
        self.dat[new_sentinel as usize].set_next(old_head);
    }
    fn alloc(&mut self, offsets: &[i32]) -> i32 {
        let sentinel = self.sentinel();
        let mut base = self.dat[sentinel as usize].next();
        'outer: while base != sentinel as i32 {
            for offset in offsets {
                let offset = (base + offset) as usize;
                if offset >= self.dat.len() {
                    break 'outer;
                }
                if self.dat[offset].used() {
                    base = self.dat[base as usize].next();
                    continue 'outer;
                }
            }
            break;
        }
        if base == sentinel as i32 {
            self.extend();
            base = self.dat[self.sentinel() as usize].next();
        }
        let max_offset = (base + *offsets.last().unwrap()) as usize;
        while max_offset >= self.dat.len() {
            self.extend();
        }
        for offset in offsets {
            self.use_(base + offset);
        }
        base
    }
    fn cleanup(mut self) -> Dat {
        let mut end = self.sentinel();
        while self.dat[end as usize].prev() == end - 1 {
            end -= 1;
        }
        let mut hole = self.dat[end as usize].prev();
        while hole != self.sentinel() {
            let prev = self.dat[hole as usize].prev();
            self.dat[hole as usize] = DatEntry {
                base: 0,
                check: hole,
            };
            hole = prev;
        }
        self.dat.drain(end as usize..);
        self.dat.shrink_to_fit();
        Dat { entries: self.dat }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dat_builder() {
        let dat = Dat::build(vec![("hit", 42), ("high", 43), ("test", 44)]);
        assert_eq!(
            dat.descendant(dat.root(), "hit").map(|x| dat.base(x)),
            Some(42)
        );
        assert_eq!(
            dat.descendant(dat.root(), "high").map(|x| dat.base(x)),
            Some(43)
        );
        assert_eq!(
            dat.descendant(dat.root(), "test").map(|x| dat.base(x)),
            Some(44)
        );
        assert_eq!(dat.descendant(dat.root(), "hix").map(|x| dat.base(x)), None);
        assert_eq!(dat.descendant(dat.root(), "x").map(|x| dat.base(x)), None);
    }
}
