#![allow(missing_docs)]

use embedded_hal::digital::v2::{InputPin, OutputPin};

use crate::layout::Event;

pub trait StateTracker {
    type State: PartialEq + Copy;
    fn get_state(&self) -> Self::State;
    fn default_state(&self) -> Self::State;
    fn emit_event(&self, last: &Self::State, now: &Self::State) -> Option<Event>;
}

impl StateTracker for () {
    type State = bool;
    fn get_state(&self) -> Self::State {
        false
    }
    fn default_state(&self) -> Self::State {
        false
    }
    fn emit_event(&self, _: &Self::State, _: &Self::State) -> Option<Event> { None }
}

pub struct DebouncedMatrix<C, R, T, const CS: usize, const RS: usize, const B: u32>
where
    C: InputPin,
    R: OutputPin,
    T: StateTracker,
{
    cols: [C; CS],
    rows: [R; RS],

    // Last known good state
    current: [u32; RS],
    // State currently being debounced
    new: [u32; RS],
    since: u32,
    tracked: T,
    last_tracked: T::State,
    last_stable_tracked: T::State,
}

impl<C, R, T, E, const CS: usize, const RS: usize, const B: u32> DebouncedMatrix<C, R, T, CS, RS, B>
where
    C: InputPin<Error = E>,
    R: OutputPin<Error = E>,
    T: StateTracker,
{
    pub fn new(cols: [C; CS], rows: [R; RS], tracked: T) -> Result<Self, E>
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
            last_tracked: tracked.default_state(),
            last_stable_tracked: tracked.default_state(),
            tracked,
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
        let mut pressed_now = [0; RS];
        for (ri, row) in (&mut self.rows).iter_mut().enumerate() {
            row.set_low()?;
            for (ci, col) in (&self.cols).iter().enumerate() {
                if col.is_low()? {
                    pressed_now[ri] |= 1 << ci;
                }
            }
            row.set_high()?;
        }

        let tracked_now = self.tracked.get_state();

        if pressed_now == self.current && tracked_now == self.last_stable_tracked {
            self.since = 0;
            return Ok(false);
        }
        if self.new != pressed_now || self.last_tracked != tracked_now {
            self.new = pressed_now;
            self.last_tracked = tracked_now;
            self.since = 1;
        } else {
            self.since += 1;
        }

        if self.since > B {
            core::mem::swap(&mut self.current, &mut self.new);
            core::mem::swap(&mut self.last_stable_tracked, &mut self.last_tracked);
            self.since = 0;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn scan(&mut self) -> Result<Option<impl Iterator<Item = Event> + '_>, E> {
        if self.update()? {
            let iter = 
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
                    })
                    .chain(self.tracked.emit_event(&self.last_tracked, &self.last_stable_tracked));
            Ok(Some(iter))
        } else {
            Ok(None)
        }
    }
}
