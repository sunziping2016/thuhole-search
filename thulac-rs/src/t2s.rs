use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Read, Result, Seek},
    mem::size_of,
    slice,
};

pub struct T2S {
    t2s: HashMap<char, char>,
}

impl T2S {
    pub fn load<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let len = reader.stream_len()? as usize;
        if len % (2 * size_of::<char>()) != 0 {
            return Err(Error::new(ErrorKind::Other, "file size unexpected"));
        }
        let count = len / 2 / size_of::<char>();
        let mut tra = vec!['\0'; count];
        let mut sim = vec!['\0'; count];
        reader.read_exact(unsafe {
            slice::from_raw_parts_mut(tra.as_mut_ptr() as *mut u8, len / 2)
        })?;
        reader.read_exact(unsafe {
            slice::from_raw_parts_mut(sim.as_mut_ptr() as *mut u8, len / 2)
        })?;
        Ok(Self {
            t2s: tra.into_iter().zip(sim.into_iter()).collect(),
        })
    }
    pub fn process<'a, I: IntoIterator<Item = char>>(
        &'a self,
        iter: I,
    ) -> T2SPipeline<'a, I::IntoIter> {
        T2SPipeline {
            inner: iter.into_iter(),
            t2s: &self.t2s,
        }
    }
}

pub struct T2SPipeline<'a, I> {
    inner: I,
    t2s: &'a HashMap<char, char>,
}

impl<'a, I> Iterator for T2SPipeline<'a, I>
where
    I: Iterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|x| self.t2s.get(&x).copied().unwrap_or(x))
    }
}
