#![allow(missing_docs)]

use embedded_hal::digital::v2::{InputPin, OutputPin};

use crate::layout::Event;

pub struct DebouncedMatrix<C, R, const CS: usize, const RS: usize, const B: u32>
where
    C: InputPin,
    R: OutputPin,
{
    cols: [C; CS],
    rows: [R; RS],

    // Last known good state
    current: [u32; RS],
    // State currently being debounced
    new: [u32; RS],
    since: u32,
}

impl<C, R, E, const CS: usize, const RS: usize, const B: u32> DebouncedMatrix<C, R, CS, RS, B>
where
    C: InputPin<Error = E>,
    R: OutputPin<Error = E>,
{
    pub fn new(cols: [C; CS], rows: [R; RS]) -> Result<Self, E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        let mut res = Self {
            cols,
            rows,
            current: [0; RS],
            new: [0; RS],
            since: 0,
        };
        res.clear()?;
        Ok(res)
    }

    fn clear(&mut self) -> Result<(), E> {
        for r in self.rows.iter_mut() {
            r.set_high()?;
        }
        Ok(())
    }

    fn update(&mut self) -> Result<bool, E> {
        let mut pressed = [0; RS];
        for (ri, row) in (&mut self.rows).iter_mut().enumerate() {
            row.set_low()?;
            for (ci, col) in (&self.cols).iter().enumerate() {
                if col.is_low()? {
                    pressed[ri] |= 1 << ci;
                }
            }
            row.set_high()?;
        }

        if pressed == self.current {
            self.since = 0;
            return Ok(false);
        }
        if self.new != pressed {
            self.new = pressed;
            self.since = 1;
        } else {
            self.since += 1;
        }

        if self.since > B {
            core::mem::swap(&mut self.current, &mut self.new);
            self.since = 0;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn scan(&mut self) -> Result<Option<impl Iterator<Item = Event> + '_ >, E> {
        if self.update()? {
            Ok(Some(
                self.new
                    .iter()
                    .zip(self.current.iter())
                    .enumerate()
                    .flat_map(move |(i, (o, n))| {
                        (0..u32::BITS).filter_map(move |b| match (o & (1 << b), n & (1 << b)) {
                            (0, 1..=u32::MAX) => Some(Event::Press(i as u8, b as u8)),
                            (1..=u32::MAX, 0) => Some(Event::Release(i as u8, b as u8)),
                            _ => None,
                        })
                    }),
            ))
        } else {
            Ok(None)
        }
    }
}