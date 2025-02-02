//! Layout management.

/// A procedural macro to generate [Layers](type.Layers.html)
/// ## Syntax
/// Items inside the macro are converted to Actions as such:
/// - [`Action::KeyCode`]: Idents are automatically understood as keycodes: `A`, `RCtrl`, `Space`
///     - Punctuation, numbers and other literals that aren't special to the rust parser are converted
///       to KeyCodes as well: `,` becomes `KeyCode::Commma`, `2` becomes `KeyCode::Kb2`, `/` becomes `KeyCode::Slash`
///     - Characters which require shifted keys are converted to `Action::MultipleKeyCodes(&[LShift, <character>])`:
///       `!` becomes `Action::MultipleKeyCodes(&[LShift, Kb1])` etc
///     - Characters special to the rust parser (parentheses, brackets, braces, quotes, apostrophes, underscores, backslashes and backticks)
///       left alone cause parsing errors and as such have to be enclosed by apostrophes: `'['` becomes `KeyCode::LBracket`,
///       `'\''` becomes `KeyCode::Quote`, `'\\'` becomes `KeyCode::BSlash`
/// - [`Action::NoOp`]: Lowercase `n`
/// - [`Action::Trans`]: Lowercase `t`
/// - [`Action::Layer`]: A number in parentheses: `(1)`, `(4 - 2)`, `(0x4u8 as usize)`
/// - [`Action::MultipleActions`]: Actions in brackets: `[LCtrl S]`, `[LAlt LCtrl C]`, `[(2) B {Action::NoOp}]`
/// - Other `Action`s: anything in braces (`{}`) is copied unchanged to the final layout - `{ Action::Custom(42) }`
///   simply becomes `Action::Custom(42)`
///
/// **Important note**: comma (`,`) is a keycode on its own, and can't be used to separate keycodes as one would have
/// to do when not using a macro.
///
/// ## Usage example:
/// Example layout for a 4x12 split keyboard:
/// ```
/// use keyberon::action::Action;
/// use keyberon::layout::{Layers, NoCustom};
/// static DLAYER: Action = Action::DefaultLayer(5);
///
/// pub static LAYERS: Layers<NoCustom, 12, 4, 2> = keyberon::layout::layout! {
///     {
///         [ Tab    Q W E R T   Y U I O P BSpace ]
///         [ LCtrl  A S D F G   H J K L ; Quote  ]
///         [ LShift Z X C V B   N M , . / Escape ]
///         [ n n LGui {DLAYER} Space Escape   BSpace Enter (1) RAlt n n ]
///     }
///     {
///         [ Tab    1 2 3 4 5   6 7 8 9 0 BSpace  ]
///         [ LCtrl  ! @ # $ %   ^ & * '(' ')' -   ]
///         [ LShift n n n n n   n n n n n [LAlt A]]
///         [ n n LGui (2) t t   t t t RAlt n n    ]
///     }
///     // ...
/// };
/// ```
pub use keyberon_macros::layout;
pub use keyberon_macros::*;

use crate::action::{Action, HoldTapConfig};
use crate::key_code::KeyCode;
use arraydeque::ArrayDeque;
use heapless::Vec;

use State::*;

/// The Layers type.
///
/// `Layers` type is an array of layers which contain the description
/// of actions on the switch matrix. For example `layers[1][2][3]`
/// corresponds to the key on the first layer, row 2, column 3.
/// The generic parameters are in order: The type contained in custom actions,
/// the number of columns, rows and layers.
/// If no custom actions are used the first parameter should be specified as
/// `keyberon::layout::NoCustom` (or `core::convert::Infallible`).
pub type Layers<T, const C: usize, const R: usize, const L: usize> = [[[Action<T>; C]; R]; L];

type Deque = ArrayDeque<[Stacked; 16], arraydeque::behavior::Wrapping>;

/// Indicates that the layout doesn't contain user-defined actions ([Action::Custom])
pub type NoCustom = core::convert::Infallible;

/// The layout manager. It takes `Event`s and `tick`s as input, and
/// generate keyboard reports.
pub struct Layout<T, const C: usize, const R: usize, const L: usize>
where
    T: 'static,
{
    layers: &'static [[[Action<T>; C]; R]; L],
    default_layer: usize,
    states: Vec<State<T>, 64>,
    waiting: Option<WaitingState<T>>,
    deque: Deque,
}

/// An event on the key matrix.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    /// Press event with coordinates (i, j).
    Press(u8, u8),
    /// Release event with coordinates (i, j).
    Release(u8, u8),
}
impl Event {
    /// Returns the coordinates (i, j) of the event.
    pub fn coord(self) -> (u8, u8) {
        match self {
            Event::Press(i, j) => (i, j),
            Event::Release(i, j) => (i, j),
        }
    }

    /// Transforms the coordinates of the event.
    ///
    /// # Example
    ///
    /// ```
    /// # use keyberon::layout::Event;
    /// assert_eq!(
    ///     Event::Press(3, 10),
    ///     Event::Press(3, 1).transform(|i, j| (i, 11 - j)),
    /// );
    /// ```
    pub fn transform(self, f: impl FnOnce(u8, u8) -> (u8, u8)) -> Self {
        match self {
            Event::Press(i, j) => {
                let (i, j) = f(i, j);
                Event::Press(i, j)
            }
            Event::Release(i, j) => {
                let (i, j) = f(i, j);
                Event::Release(i, j)
            }
        }
    }

    /// Returns `true` if the event is a key press.
    pub fn is_press(self) -> bool {
        match self {
            Event::Press(..) => true,
            Event::Release(..) => false,
        }
    }

    /// Returns `true` if the event is a key release.
    pub fn is_release(self) -> bool {
        match self {
            Event::Release(..) => true,
            Event::Press(..) => false,
        }
    }
}

/// Event from custom action.
#[derive(Debug, PartialEq, Eq)]
pub enum CustomEvent<T: 'static> {
    /// No custom action.
    NoEvent,
    /// The given custom action key is pressed.
    Press(&'static T),
    /// The given custom action key is released.
    Release(&'static T),
}
impl<T> CustomEvent<T> {
    /// Update an event according to a new event.
    ///
    ///The event can only be modified in the order `NoEvent < Press <
    /// Release`
    fn update(&mut self, e: Self) {
        use CustomEvent::*;
        match (&e, &self) {
            (Release(_), NoEvent) | (Release(_), Press(_)) => *self = e,
            (Press(_), NoEvent) => *self = e,
            _ => (),
        }
    }
}
impl<T> Default for CustomEvent<T> {
    fn default() -> Self {
        CustomEvent::NoEvent
    }
}

#[derive(Debug, Eq, PartialEq)]
enum State<T: 'static> {
    NormalKey { keycode: KeyCode, coord: (u8, u8) },
    LayerModifier { value: usize, coord: (u8, u8) },
    Custom { value: &'static T, coord: (u8, u8) },
}
impl<T> Copy for State<T> {}
impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> State<T> {
    fn keycode(&self) -> Option<KeyCode> {
        match self {
            NormalKey { keycode, .. } => Some(*keycode),
            _ => None,
        }
    }
    fn release(&self, c: (u8, u8), custom: &mut CustomEvent<T>) -> Option<Self> {
        match *self {
            NormalKey { coord, .. } | LayerModifier { coord, .. } if coord == c => None,
            Custom { value, coord } if coord == c => {
                custom.update(CustomEvent::Release(value));
                None
            }
            _ => Some(*self),
        }
    }
    fn get_layer(&self) -> Option<usize> {
        match self {
            LayerModifier { value, .. } => Some(*value),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct WaitingState<T: 'static> {
    coord: (u8, u8),
    timeout: u16,
    delay: u16,
    hold: &'static Action<T>,
    tap: &'static Action<T>,
    config: HoldTapConfig,
}
enum WaitingAction {
    Hold,
    Tap,
    NoOp,
}
impl<T> WaitingState<T> {
    fn tick(&mut self, stacked: &Deque) -> WaitingAction {
        self.timeout = self.timeout.saturating_sub(1);
        match self.config {
            HoldTapConfig::Default => (),
            HoldTapConfig::HoldOnOtherKeyPress => {
                if stacked.iter().any(|s| s.event.is_press()) {
                    return WaitingAction::Hold;
                }
            }
            HoldTapConfig::PermissiveHold => {
                for (x, s) in stacked.iter().enumerate() {
                    if s.event.is_press() {
                        let (i, j) = s.event.coord();
                        let target = Event::Release(i, j);
                        if stacked.iter().skip(x + 1).any(|s| s.event == target) {
                            return WaitingAction::Hold;
                        }
                    }
                }
            }
        }
        if let Some(&Stacked { since, .. }) = stacked
            .iter()
            .find(|s| self.is_corresponding_release(&s.event))
        {
            if self.timeout >= self.delay - since {
                WaitingAction::Tap
            } else {
                WaitingAction::Hold
            }
        } else if self.timeout == 0 {
            WaitingAction::Hold
        } else {
            WaitingAction::NoOp
        }
    }
    fn is_corresponding_release(&self, event: &Event) -> bool {
        matches!(event, Event::Release(i, j) if (*i, *j) == self.coord)
    }
}

#[derive(Debug)]
struct Stacked {
    event: Event,
    since: u16,
}
impl From<Event> for Stacked {
    fn from(event: Event) -> Self {
        Stacked { event, since: 0 }
    }
}
impl Stacked {
    fn tick(&mut self) {
        self.since = self.since.saturating_add(1);
    }
}

impl<T: 'static, const C: usize, const R: usize, const L: usize> Layout<T, C, R, L> {
    /// Creates a new `Layout` object.
    pub fn new(layers: &'static [[[Action<T>; C]; R]; L]) -> Self {
        Self {
            layers,
            default_layer: 0,
            states: Vec::new(),
            waiting: None,
            deque: ArrayDeque::new(),
        }
    }
    /// Iterates on the key codes of the current state.
    pub fn keycodes(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.states.iter().filter_map(State::keycode)
    }
    fn waiting_into_hold(&mut self) -> CustomEvent<T> {
        if let Some(w) = &self.waiting {
            let hold = w.hold;
            let coord = w.coord;
            self.waiting = None;
            self.do_action(hold, coord, 0)
        } else {
            CustomEvent::NoEvent
        }
    }
    fn waiting_into_tap(&mut self) -> CustomEvent<T> {
        if let Some(w) = &self.waiting {
            let tap = w.tap;
            let coord = w.coord;
            self.waiting = None;
            self.do_action(tap, coord, 0)
        } else {
            CustomEvent::NoEvent
        }
    }
    /// A time event.
    ///
    /// This method must be called regularly, typically every millisecond.
    ///
    /// Returns the corresponding `CustomEvent`, allowing to manage
    /// custom actions thanks to the `Action::Custom` variant.
    pub fn tick(&mut self) -> CustomEvent<T> {
        //self.states = self.states.iter().filter_map(State::tick).collect();
        self.deque.iter_mut().for_each(Stacked::tick);
        match &mut self.waiting {
            Some(w) => match w.tick(&self.deque) {
                WaitingAction::Hold => self.waiting_into_hold(),
                WaitingAction::Tap => self.waiting_into_tap(),
                WaitingAction::NoOp => CustomEvent::NoEvent,
            },
            None => match self.deque.pop_front() {
                Some(s) => self.unstack(s),
                None => CustomEvent::NoEvent,
            },
        }
    }
    fn unstack(&mut self, stacked: Stacked) -> CustomEvent<T> {
        use Event::*;
        match stacked.event {
            Release(i, j) => {
                let mut custom = CustomEvent::NoEvent;
                //self.states = self
                //    .states
                //    .iter()
                //    .filter_map(|s| s.release((i, j), &mut custom))
                //    .collect();
                self.states.map_retain(|s| s.release((i, j), &mut custom));
                custom
            }
            Press(i, j) => {
                let action = self.press_as_action((i, j), self.current_layer());
                self.do_action(action, (i, j), stacked.since)
            }
        }
    }
    /// Register a key event.
    pub fn event(&mut self, event: Event) {
        if let Some(stacked) = self.deque.push_back(event.into()) {
            self.waiting_into_hold();
            self.unstack(stacked);
        }
    }
    fn press_as_action(&self, coord: (u8, u8), layer: usize) -> &'static Action<T> {
        use crate::action::Action::*;
        let action = self
            .layers
            .get(layer)
            .and_then(|l| l.get(coord.0 as usize))
            .and_then(|l| l.get(coord.1 as usize));
        match action {
            None => &NoOp,
            Some(Trans) => {
                if layer != self.default_layer {
                    self.press_as_action(coord, self.default_layer)
                } else {
                    &NoOp
                }
            }
            Some(action) => action,
        }
    }
    fn do_action(
        &mut self,
        action: &'static Action<T>,
        coord: (u8, u8),
        delay: u16,
    ) -> CustomEvent<T> {
        assert!(self.waiting.is_none());
        use Action::*;
        match action {
            NoOp | Trans => (),
            HoldTap {
                timeout,
                hold,
                tap,
                config,
                ..
            } => {
                let waiting: WaitingState<T> = WaitingState {
                    coord,
                    timeout: *timeout,
                    delay,
                    hold,
                    tap,
                    config: *config,
                };
                self.waiting = Some(waiting);
            }
            &KeyCode(keycode) => {
                let _ = self.states.push(NormalKey { coord, keycode });
            }
            &MultipleKeyCodes(v) => {
                for &keycode in v {
                    let _ = self.states.push(NormalKey { coord, keycode });
                }
            }
            &MultipleActions(v) => {
                let mut custom = CustomEvent::NoEvent;
                for action in v {
                    custom.update(self.do_action(action, coord, delay));
                }
                return custom;
            }
            &Layer(value) => {
                let _ = self.states.push(LayerModifier { value, coord });
            }
            DefaultLayer(value) => {
                self.set_default_layer(*value);
            }
            Custom(value) => {
                if self.states.push(State::Custom { value, coord }).is_ok() {
                    return CustomEvent::Press(value);
                }
            }
        }
        CustomEvent::NoEvent
    }

    /// Obtain the index of the current active layer
    pub fn current_layer(&self) -> usize {
        let mut iter = self.states.iter().filter_map(State::get_layer);
        let mut layer = match iter.next() {
            None => self.default_layer,
            Some(l) => l,
        };
        for l in iter {
            layer += l;
        }
        layer
    }

    /// Sets the default layer for the layout
    pub fn set_default_layer(&mut self, value: usize) {
        if value < self.layers.len() {
            self.default_layer = value
        }
    }
}

trait MapRetain<T> {
    fn map_retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> Option<T>;
}

impl<T, const S: usize> MapRetain<T> for Vec<T, { S }> {
    fn map_retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> Option<T>,
    {
        let mut processed = 0;
        while processed < self.len() {
            let res = f(&self[processed]);
            match res {
                Some(t) => {
                    self[processed] = t;
                    processed += 1;
                }
                None => unsafe {
                    self.swap_remove_unchecked(processed);
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::{Event::*, Layout, *};
    use crate::action::Action::*;
    use crate::action::HoldTapConfig;
    use crate::action::{k, l, m};
    use crate::key_code::KeyCode;
    use crate::key_code::KeyCode::*;
    use std::collections::BTreeSet;

    #[track_caller]
    fn assert_keys(expected: &[KeyCode], iter: impl Iterator<Item = KeyCode>) {
        let expected: BTreeSet<_> = expected.iter().copied().collect();
        let tested = iter.collect();
        assert_eq!(expected, tested);
    }

    #[test]
    fn basic_hold_tap() {
        static LAYERS: Layers<NoCustom, 2, 1, 2> = [
            [[
                HoldTap {
                    timeout: 200,
                    hold: &l(1),
                    tap: &k(Space),
                    config: HoldTapConfig::Default,
                    tap_hold_interval: 0,
                },
                HoldTap {
                    timeout: 200,
                    hold: &k(LCtrl),
                    tap: &k(Enter),
                    config: HoldTapConfig::Default,
                    tap_hold_interval: 0,
                },
            ]],
            [[Trans, m(&[LCtrl, Enter])]],
        ];
        let mut layout = Layout::new(&LAYERS);
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..197 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl, Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn hold_tap_interleaved_timeout() {
        static LAYERS: Layers<NoCustom, 2, 1, 1> = [[[
            HoldTap {
                timeout: 200,
                hold: &k(LAlt),
                tap: &k(Space),
                config: HoldTapConfig::Default,
                tap_hold_interval: 0,
            },
            HoldTap {
                timeout: 20,
                hold: &k(LCtrl),
                tap: &k(Enter),
                config: HoldTapConfig::Default,
                tap_hold_interval: 0,
            },
        ]]];
        let mut layout = Layout::new(&LAYERS);
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        for _ in 0..15 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        for _ in 0..10 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[Space], layout.keycodes());
        }
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space, LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn hold_on_press() {
        static LAYERS: Layers<NoCustom, 2, 1, 1> = [[[
            HoldTap {
                timeout: 200,
                hold: &k(LAlt),
                tap: &k(Space),
                config: HoldTapConfig::HoldOnOtherKeyPress,
                tap_hold_interval: 0,
            },
            k(Enter),
        ]]];
        let mut layout = Layout::new(&LAYERS);

        // Press another key before timeout
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Press another key after timeout
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn permissive_hold() {
        static LAYERS: Layers<NoCustom, 2, 1, 1> = [[[
            HoldTap {
                timeout: 200,
                hold: &k(LAlt),
                tap: &k(Space),
                config: HoldTapConfig::PermissiveHold,
                tap_hold_interval: 0,
            },
            k(Enter),
        ]]];
        let mut layout = Layout::new(&LAYERS);

        // Press and release another key before timeout
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn multiple_actions() {
        static LAYERS: Layers<NoCustom, 2, 1, 2> = [
            [[MultipleActions(&[l(1), k(LShift)]), k(F)]],
            [[Trans, k(E)]],
        ];
        let mut layout = Layout::new(&LAYERS);
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, E], layout.keycodes());
        layout.event(Release(0, 1));
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn multiple_custom_actions() {
        static LAYERS: Layers<u8, 1, 1, 1> = [[[MultipleActions(&[
            Action::Custom(1),
            Action::Custom(2),
            Action::Custom(3),
        ])]]];
        let mut layout = Layout::new(&LAYERS);

        // Custom event
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::Press(&1), layout.tick());
        assert_keys(&[], layout.keycodes());

        // nothing more
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // release custom
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::Release(&1), layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn custom() {
        static LAYERS: Layers<u8, 1, 1, 1> = [[[Action::Custom(42)]]];
        let mut layout = Layout::new(&LAYERS);
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Custom event
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::Press(&42), layout.tick());
        assert_keys(&[], layout.keycodes());

        // nothing more
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // release custom
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::Release(&42), layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn test_map_retain() {
        let mut vec = Vec::<u32, 10>::new();
        vec.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        // Remove every odd number
        vec.map_retain(|n| if n % 2 == 0 { Some(*n) } else { None });

        // Check that every number that's left is even
        assert!(vec.iter().all(|n| n % 2 == 0));
    }
}
